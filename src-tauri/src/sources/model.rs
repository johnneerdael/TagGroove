//! Provider-agnostic release model shared by every online or local tag source.
use serde::{Deserialize, Serialize};
use tag_core::Fields;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Musicbrainz,
    Discogs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    ArtistAlbum,
    Text,
    Barcode,
    CatalogNumber,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SearchQuery {
    pub mode: SearchMode,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub album: String,
    #[serde(default)]
    pub text: String,
}

/// One row of the search results list.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub year: String,
    pub country: String,
    pub label: String,
    pub catalog_number: String,
    pub format: String,
    pub track_count: Option<u32>,
    pub score: Option<u32>,
    pub barcode: String,
    /// Small preview image, fetched on demand through `tag_source_image`.
    pub thumb: Option<String>,
    pub url: String,
}

/// A complete release: album-level fields apply to every matched file and
/// each track's fields override them for its file.
#[derive(Clone, Debug, Serialize)]
pub struct SourceRelease {
    pub provider: Provider,
    pub id: String,
    pub url: String,
    pub title: String,
    pub artist: String,
    pub fields: Fields,
    pub tracks: Vec<SourceTrack>,
    pub covers: Vec<SourceCover>,
    /// More artwork can be listed with `tag_source_covers`.
    pub more_covers: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SourceTrack {
    /// Position as printed by the source, for example `1`, `2-03` or `A1`.
    pub position: String,
    pub disc: u32,
    pub number: u32,
    pub title: String,
    pub artist: String,
    pub length: Option<f64>,
    pub fields: Fields,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SourceCover {
    pub url: String,
    pub thumb: Option<String>,
    pub kind: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub(crate) fn put(fields: &mut Fields, key: &str, values: Vec<String>) {
    let mut unique: Vec<String> = Vec::new();
    for value in values.into_iter().map(|v| v.trim().to_owned()) {
        if !value.is_empty() && !unique.contains(&value) {
            unique.push(value);
        }
    }
    if !unique.is_empty() {
        fields.insert(key.to_owned(), unique);
    }
}

pub(crate) fn put_one(fields: &mut Fields, key: &str, value: impl Into<String>) {
    put(fields, key, vec![value.into()]);
}

/// Number tracks per disc and add `TRACK`/`DISCNUMBER` with totals.
pub(crate) fn number_tracks(tracks: &mut [SourceTrack]) {
    let discs = tracks
        .iter()
        .map(|t| t.disc)
        .collect::<std::collections::BTreeSet<_>>();
    let disc_total = discs.len().max(1);
    for (disc_index, disc) in discs.iter().enumerate() {
        let total = tracks.iter().filter(|t| t.disc == *disc).count();
        for (index, track) in tracks.iter_mut().filter(|t| t.disc == *disc).enumerate() {
            track.number = index as u32 + 1;
            put_one(&mut track.fields, "TRACK", format!("{}/{total}", index + 1));
            put_one(
                &mut track.fields,
                "DISCNUMBER",
                format!("{}/{disc_total}", disc_index + 1),
            );
            track.disc = disc_index as u32 + 1;
        }
    }
}

pub(crate) fn year(date: &str) -> String {
    let digits: String = date.trim().chars().take(4).collect();
    if digits.len() == 4 && digits.chars().all(|c| c.is_ascii_digit()) && digits != "0000" {
        digits
    } else {
        String::new()
    }
}

/// `Electronic`, `French House`: providers use mixed casing for genres.
pub(crate) fn title_case(value: &str) -> String {
    value
        .split(' ')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect())
                .unwrap_or_default()
        })
        .collect::<Vec<String>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(disc: u32) -> SourceTrack {
        SourceTrack {
            position: String::new(),
            disc,
            number: 0,
            title: String::new(),
            artist: String::new(),
            length: None,
            fields: Fields::new(),
        }
    }

    #[test]
    fn numbering_restarts_per_disc_and_compacts_disc_numbers() {
        let mut tracks = vec![track(1), track(1), track(3)];
        number_tracks(&mut tracks);
        let values: Vec<_> = tracks
            .iter()
            .map(|t| {
                (
                    t.fields["TRACK"][0].as_str(),
                    t.fields["DISCNUMBER"][0].as_str(),
                )
            })
            .collect();
        assert_eq!(values, [("1/2", "1/2"), ("2/2", "1/2"), ("1/1", "2/2")]);
        assert_eq!((tracks[2].disc, tracks[2].number), (2, 1));
    }

    #[test]
    fn helpers_trim_deduplicate_and_normalise() {
        let mut fields = Fields::new();
        put(
            &mut fields,
            "LABEL",
            vec![" A ".into(), "A".into(), String::new()],
        );
        put(&mut fields, "EMPTY", vec![" ".into()]);
        assert_eq!(fields.get("LABEL").unwrap(), &vec!["A".to_string()]);
        assert!(!fields.contains_key("EMPTY"));
        assert_eq!(year("1999-03-02"), "1999");
        assert_eq!(year("0000"), "");
        assert_eq!(year("19"), "");
        assert_eq!(title_case("french house"), "French House");
    }
}
