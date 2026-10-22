//! Application keys of online services, built into release builds from the
//! build environment (CI passes repository secrets as `OPENTAG_*` variables, and
//! as `TAURI_OPENTAG_*` aliases for the mobile builds: Tauri forwards only
//! `TAURI`-prefixed variables across the Gradle/Xcode boundary, so the plain
//! names do not reliably reach their Cargo builds).
//! A build without them compiles normally: Discogs then falls back to a
//! personal token, fanart.tv waits for a key the user enters in Tag Sources →
//! API keys…, and AcoustID and Last.fm are unavailable (nobody enters keys for
//! those two: AcoustID's personal key is only for submitting fingerprints,
//! which TagGroove never does). The values never reach the webview, which only
//! learns whether a key exists.

/// A built-in key, or `None` when the build had none (or an empty one). The plain `OPENTAG_*`
/// value wins; the `TAURI_OPENTAG_*` alias is used when it is absent or empty.
fn built_in(primary: Option<&'static str>, alias: Option<&'static str>) -> Option<&'static str> {
    [primary, alias]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
}

/// TagGroove's Discogs application consumer key and secret.
pub fn discogs_app() -> Option<(&'static str, &'static str)> {
    Some((
        built_in(
            option_env!("OPENTAG_DISCOGS_CONSUMER_KEY"),
            option_env!("TAURI_OPENTAG_DISCOGS_CONSUMER_KEY"),
        )?,
        built_in(
            option_env!("OPENTAG_DISCOGS_CONSUMER_SECRET"),
            option_env!("TAURI_OPENTAG_DISCOGS_CONSUMER_SECRET"),
        )?,
    ))
}

/// The application key (`client`) for AcoustID lookups.
pub fn acoustid() -> Option<&'static str> {
    built_in(
        option_env!("OPENTAG_ACOUSTID_API_KEY"),
        option_env!("TAURI_OPENTAG_ACOUSTID_API_KEY"),
    )
}

/// TagGroove's own Last.fm application key. Users never enter one.
pub fn lastfm() -> Option<&'static str> {
    built_in(
        option_env!("OPENTAG_LASTFM_API_KEY"),
        option_env!("TAURI_OPENTAG_LASTFM_API_KEY"),
    )
}

/// The Last.fm shared secret that signs the desktop sign-in. It never reaches the webview.
pub fn lastfm_secret() -> Option<&'static str> {
    built_in(
        option_env!("OPENTAG_LASTFM_SHARED_SECRET"),
        option_env!("TAURI_OPENTAG_LASTFM_SHARED_SECRET"),
    )
}

/// The fanart.tv project API key.
pub fn fanart() -> Option<&'static str> {
    built_in(
        option_env!("OPENTAG_FANARTTV_API_KEY"),
        option_env!("TAURI_OPENTAG_FANARTTV_API_KEY"),
    )
}

/// How Discogs requests authenticate: the user's personal token first, then
/// TagGroove's application key and secret. Missing credentials must not be sent.
pub fn discogs_auth(token: &str, app: Option<(&str, &str)>) -> super::http::Auth {
    use super::http::Auth;
    let token = token.trim();
    if !token.is_empty() {
        return Auth::DiscogsToken(token.to_owned());
    }
    match app {
        Some((key, secret)) => Auth::DiscogsApp {
            key: key.to_owned(),
            secret: secret.to_owned(),
        },
        None => Auth::None,
    }
}

/// Only a user's token qualifies for the authenticated-user budget. The
/// application's key shares the conservative 25 requests/minute tier.
pub fn discogs_spacing(auth: &super::http::Auth) -> u64 {
    match auth {
        super::http::Auth::DiscogsToken(token) if !token.trim().is_empty() => 1100,
        _ => 2500,
    }
}

#[cfg(test)]
mod tests {
    use super::super::http::Auth;
    use super::*;

    #[test]
    fn empty_build_values_count_as_missing() {
        assert_eq!(built_in(None, None), None);
        assert_eq!(built_in(Some("  "), Some("")), None);
        assert_eq!(built_in(Some(" abc "), None), Some("abc"));
    }

    #[test]
    fn the_tauri_alias_fills_in_when_the_plain_name_did_not_cross_the_build_boundary() {
        // The mobile builds see only `TAURI_OPENTAG_*`; the plain name wins when both are set.
        assert_eq!(built_in(None, Some(" mobile ")), Some("mobile"));
        assert_eq!(built_in(Some(""), Some("mobile")), Some("mobile"));
        assert_eq!(built_in(Some("plain"), Some("mobile")), Some("plain"));
    }

    /// Compiled with only the alias set (`TAURI_OPENTAG_ACOUSTID_API_KEY=x cargo test ...`), the
    /// key must still resolve; in any other build this checks nothing.
    #[test]
    fn a_build_that_only_has_the_alias_still_has_the_key() {
        if option_env!("OPENTAG_ACOUSTID_API_KEY").is_none()
            && option_env!("TAURI_OPENTAG_ACOUSTID_API_KEY").is_some_and(|v| !v.trim().is_empty())
        {
            assert!(acoustid().is_some());
        }
    }

    #[test]
    fn discogs_prefers_the_personal_token_then_the_app_key() {
        assert!(matches!(
            discogs_auth(" mine ", Some(("k", "s"))),
            Auth::DiscogsToken(token) if token == "mine"
        ));
        assert!(matches!(
            discogs_auth("", Some(("k", "s"))),
            Auth::DiscogsApp { key, secret } if key == "k" && secret == "s"
        ));
        assert!(matches!(discogs_auth("", None), Auth::None));
    }

    #[test]
    fn discogs_application_and_user_credentials_have_distinct_budgets() {
        use super::super::http::Auth;
        let app = Auth::DiscogsApp {
            key: "application".into(),
            secret: "secret".into(),
        };
        assert_eq!(discogs_spacing(&app), 2500);
        assert_eq!(discogs_spacing(&Auth::None), 2500);
        assert_eq!(
            discogs_spacing(&Auth::DiscogsToken("personal".into())),
            1100
        );
        assert_eq!(discogs_spacing(&Auth::DiscogsToken(String::new())), 2500);
    }
}
