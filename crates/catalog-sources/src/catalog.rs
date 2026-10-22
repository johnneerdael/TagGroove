use crate::http::Http;
use crate::{Page, Release};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Beatport,
    Traxsource,
}
pub struct Endpoints {
    pub beatport_api: String,
    pub beatport_auth: String,
    pub traxsource: String,
}
impl Default for Endpoints {
    fn default() -> Self {
        Self {
            beatport_api: "https://api.beatport.com".into(),
            beatport_auth: "https://account.beatport.com".into(),
            traxsource: "https://www.traxsource.com".into(),
        }
    }
}
pub struct Catalog {
    pub(crate) http: Http,
    pub(crate) endpoints: Endpoints,
}
impl Catalog {
    pub fn new(http: Http) -> Self {
        Self {
            http,
            endpoints: Endpoints::default(),
        }
    }
    pub fn with_endpoints(http: Http, endpoints: Endpoints) -> Self {
        Self { http, endpoints }
    }

    fn url(base: &str, path: &str) -> Result<reqwest::Url, String> {
        reqwest::Url::parse(&format!("{}{path}", base.trim_end_matches('/')))
            .map_err(|_| "Invalid catalog endpoint".into())
    }
    async fn token(&self) -> Result<String, String> {
        let mut cache = self.http.cancellable(self.http.state.token.lock()).await?;
        if let Some((token, expires)) = cache.as_ref() {
            if *expires > std::time::Instant::now() {
                return Ok(token.clone());
            }
        }
        let url = Self::url(&self.endpoints.beatport_auth, "/o/token/")?;
        let body = self
            .http
            .post_form(
                &url,
                &[
                    ("client_id", crate::auth::CLIENT_ID),
                    ("client_secret", crate::auth::CLIENT_SECRET),
                    ("grant_type", "client_credentials"),
                ],
            )
            .await?;
        let v: serde_json::Value =
            serde_json::from_str(&body).map_err(|_| "Malformed Beatport token response")?;
        let token = v["access_token"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("Beatport token response is missing an access token")?
            .to_owned();
        let seconds = v["expires_in"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("Beatport token response is missing expiry")?
            .min(24 * 3600)
            .saturating_sub(10);
        *cache = Some((
            token.clone(),
            std::time::Instant::now() + std::time::Duration::from_secs(seconds),
        ));
        Ok(token)
    }
    async fn beatport_get(&self, path: &str) -> Result<String, String> {
        let token = self.token().await?;
        self.http
            .get(
                &Self::url(&self.endpoints.beatport_api, path)?,
                Some(&token),
                false,
            )
            .await
    }
    pub async fn search(
        &self,
        provider: Provider,
        releases: bool,
        term: &str,
        page: u32,
    ) -> Result<Page, String> {
        self.http.check()?;
        if term.trim().is_empty() {
            return Err("Enter an artist and title, release name, or search text".into());
        }
        if page > 10000 {
            return Err("Catalog page limit exceeded".into());
        }
        match provider {
            Provider::Beatport => {
                let term = term
                    .replace(['(', ')', '[', ']', ','], " ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                let mut url = Self::url(&self.endpoints.beatport_api, "/v4/catalog/search/")?;
                url.query_pairs_mut()
                    .append_pair("q", &term)
                    .append_pair("type", if releases { "releases" } else { "tracks" })
                    .append_pair("page", &(page + 1).to_string())
                    .append_pair("per_page", "50");
                let token = self.token().await?;
                crate::beatport::parse_search(
                    &self.http.get(&url, Some(&token), false).await?,
                    releases,
                )
            }
            Provider::Traxsource => {
                // Captured release grids hold 18 items; track pages hold 50. Expose the
                // application's stable 50-result pages across either native pagination.
                let per_page = if releases { 18 } else { 50 };
                let offset = page * 50;
                let first = offset / per_page;
                let last = (offset + 49) / per_page;
                let mut result = Page::default();
                for server_page in first..=last {
                    self.http.check()?;
                    let mut url = Self::url(
                        &self.endpoints.traxsource,
                        if releases {
                            "/scripts/search.php/titles"
                        } else {
                            "/scripts/search.php/tracks"
                        },
                    )?;
                    url.query_pairs_mut()
                        .append_pair("term", term)
                        .append_pair("page", &(server_page + 1).to_string())
                        .append_pair("rpc", "1");
                    let parsed = crate::traxsource::parse_search(
                        &self.http.get(&url, None, true).await?,
                        releases,
                    )?;
                    if let Some(total) = parsed.total {
                        if result.total.is_some_and(|prior| prior != total) {
                            return Err(
                                "Traxsource results changed while paging; repeat the search".into(),
                            );
                        }
                        result.total = Some(total)
                    }
                    let len = parsed.results.len();
                    let page_start = server_page * per_page;
                    result
                        .results
                        .extend(parsed.results.into_iter().enumerate().filter_map(
                            |(index, hit)| {
                                let index = page_start + index as u32;
                                (index >= offset && index < offset + 50).then_some(hit)
                            },
                        ));
                    if len < per_page as usize
                        || result
                            .total
                            .is_some_and(|total| page_start + len as u32 >= total)
                    {
                        break;
                    }
                }
                Ok(result)
            }
        }
    }
    fn identity(input: &str) -> Result<(bool, &str), String> {
        let (kind, id) = input
            .split_once(':')
            .ok_or("Catalog IDs must start with track: or release:")?;
        if !matches!(kind, "track" | "release")
            || id.is_empty()
            || !id.chars().all(|c| c.is_ascii_digit())
            || !id.parse::<u64>().is_ok_and(|n| n > 0)
        {
            return Err("Invalid catalog ID".into());
        }
        Ok((kind == "track", id))
    }
    pub async fn release(&self, provider: Provider, input: &str) -> Result<Release, String> {
        self.http.check()?;
        let normalized = normalize_id(provider, input)?;
        let (single, id) = Self::identity(&normalized)?;
        match provider {
            Provider::Traxsource => {
                let mut detail_fields = std::collections::BTreeMap::new();
                let release_id = if single {
                    let body = self
                        .http
                        .get(
                            &Self::url(
                                &self.endpoints.traxsource,
                                &format!("/scripts/builder.php/track/{id}?rpc=1"),
                            )?,
                            None,
                            true,
                        )
                        .await?;
                    let (release_id, fields) = crate::traxsource::track_detail(&body, id)?;
                    detail_fields = fields;
                    release_id
                } else {
                    id.into()
                };
                let body = self
                    .http
                    .get(
                        &Self::url(
                            &self.endpoints.traxsource,
                            &format!("/scripts/builder.php/title/{release_id}?rpc=1"),
                        )?,
                        None,
                        true,
                    )
                    .await?;
                let mut release = crate::traxsource::parse_release(&body, &release_id)?;
                if single {
                    let mut track =
                        release.tracks.into_iter().find(|t| t.id == id).ok_or(
                            "The selected Traxsource track is absent from its linked release",
                        )?;
                    track.fields.extend(detail_fields);
                    release.tracks = vec![track];
                }
                Ok(release)
            }
            Provider::Beatport => {
                let release_id = if single {
                    let body = self
                        .beatport_get(&format!("/v4/catalog/tracks/{id}"))
                        .await?;
                    let track = crate::parse_beatport_track(&body)?;
                    if track.id != id {
                        return Err("Beatport track identity mismatch".into());
                    }
                    track.release_id
                } else {
                    id.into()
                };
                let mut release = self.beatport_release(&release_id).await?;
                if single {
                    let track =
                        release.tracks.into_iter().find(|t| t.id == id).ok_or(
                            "The selected Beatport track is absent from its linked release",
                        )?;
                    release.tracks = vec![track];
                }
                Ok(release)
            }
        }
    }
    async fn beatport_release(&self, id: &str) -> Result<Release, String> {
        let body = self
            .beatport_get(&format!("/v4/catalog/releases/{id}"))
            .await?;
        let v: serde_json::Value =
            serde_json::from_str(&body).map_err(|_| "Malformed Beatport release JSON")?;
        let mut release = crate::beatport::parse_release_value(&v)?;
        if release.id != id {
            return Err("Beatport release identity mismatch".into());
        }
        let expected = v["track_count"].as_u64();
        let mut ids = std::collections::BTreeSet::new();
        let mut numbers = std::collections::BTreeSet::new();
        // Beatport's track listing is in release order but its rows carry no `number` (the
        // listing schema has none; only a single track's detail does). Without positions the
        // order is the position.
        let mut by_order = None;
        for page in 1..=100 {
            let body = self
                .beatport_get(&format!(
                    "/v4/catalog/releases/{id}/tracks?per_page=200&page={page}"
                ))
                .await?;
            let v: serde_json::Value = serde_json::from_str(&body)
                .map_err(|_| "Malformed Beatport release tracks JSON")?;
            let rows = v["results"]
                .as_array()
                .ok_or("Malformed Beatport release: missing tracklist")?;
            let by_order = *by_order.get_or_insert_with(|| {
                !rows
                    .iter()
                    .any(|row| row["number"].as_u64().is_some_and(|n| n > 0))
            });
            for row in rows {
                let mut track = crate::beatport::parse_track_value(row)?;
                if by_order {
                    track.number = u32::try_from(release.tracks.len() + 1)
                        .map_err(|_| "Beatport release has too many tracks")?;
                }
                if track.release_id != id || !ids.insert(track.id.clone()) {
                    return Err("Beatport release track identity mismatch".into());
                }
                if track.number == 0 {
                    return Err("Incomplete Beatport release track positions".into());
                }
                if !numbers.insert(track.number) {
                    return Err("Beatport release contains duplicate track positions".into());
                }
                release.tracks.push(track)
            }
            let has_next = v["next"].as_str().is_some_and(|s| !s.is_empty());
            if !has_next
                && (rows.len() < 200 || expected.is_some_and(|n| n <= release.tracks.len() as u64))
            {
                if release.tracks.is_empty()
                    || expected.is_some_and(|n| n != release.tracks.len() as u64)
                {
                    return Err("Incomplete Beatport release tracklist".into());
                }
                if numbers.iter().copied().ne(1..=release.tracks.len() as u32) {
                    return Err("Incomplete Beatport release track positions".into());
                }
                let total = release.tracks.len().to_string();
                for track in &mut release.tracks {
                    track
                        .fields
                        .insert("TRACKTOTAL".into(), vec![total.clone()]);
                }
                return Ok(release);
            }
            if rows.is_empty() {
                return Err("Incomplete Beatport release tracklist".into());
            }
        }
        Err("Beatport release exceeded pagination limit".into())
    }
}

pub fn normalize_id(provider: Provider, input: &str) -> Result<String, String> {
    let input = input.trim();
    if input.starts_with("track:") || input.starts_with("release:") {
        let (single, id) = Catalog::identity(input)?;
        let id = id.parse::<u64>().map_err(|_| "Invalid catalog ID")?;
        return Ok(format!("{}:{id}", if single { "track" } else { "release" }));
    }
    let url = reqwest::Url::parse(input)
        .map_err(|_| "Enter a track:<id>, release:<id>, or catalog track/release HTTPS address")?;
    let allowed = match provider {
        Provider::Beatport => matches!(url.host_str(), Some("www.beatport.com" | "beatport.com")),
        Provider::Traxsource => matches!(
            url.host_str(),
            Some("www.traxsource.com" | "traxsource.com")
        ),
    };
    if !allowed
        || url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return Err("Enter an HTTPS track or release address from the selected catalog without credentials or a custom port".into());
    }
    let parts = url
        .path_segments()
        .ok_or("Invalid catalog address")?
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let (kind, id) = match (provider, parts.as_slice()) {
        (Provider::Beatport, [kind, id]) if matches!(*kind, "track" | "release") => (*kind, *id),
        (Provider::Beatport, [kind, _slug, id]) if matches!(*kind, "track" | "release") => {
            (*kind, *id)
        }
        (Provider::Traxsource, [kind, id]) | (Provider::Traxsource, [kind, id, _])
            if matches!(*kind, "track" | "title") =>
        {
            (if *kind == "title" { "release" } else { "track" }, *id)
        }
        _ => return Err("Enter a track or release address from the selected catalog".into()),
    };
    let canonical = format!("{kind}:{id}");
    Catalog::identity(&canonical)?;
    Ok(format!(
        "{kind}:{}",
        id.parse::<u64>().map_err(|_| "Invalid catalog ID")?
    ))
}
