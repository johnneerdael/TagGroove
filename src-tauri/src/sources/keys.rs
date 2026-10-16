//! Application keys of online services, built into release builds from the
//! build environment (CI passes repository secrets as `OPENTAG_*` variables).
//! A build without them compiles normally: Discogs then falls back to a
//! personal token or anonymous requests, and the key-gated sources wait for a
//! key the user enters in Tag Sources → API keys…. The values never reach the
//! webview, which only learns whether a key exists.

/// A built-in key, or `None` when the build had none (or an empty one).
fn built_in(value: Option<&'static str>) -> Option<&'static str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// OpenTag's Discogs application consumer key and secret.
pub fn discogs_app() -> Option<(&'static str, &'static str)> {
    Some((
        built_in(option_env!("OPENTAG_DISCOGS_CONSUMER_KEY"))?,
        built_in(option_env!("OPENTAG_DISCOGS_CONSUMER_SECRET"))?,
    ))
}

/// The application key (`client`) for AcoustID lookups.
pub fn acoustid() -> Option<&'static str> {
    built_in(option_env!("OPENTAG_ACOUSTID_API_KEY"))
}

/// The Last.fm API key (read-only methods need no secret).
pub fn lastfm() -> Option<&'static str> {
    built_in(option_env!("OPENTAG_LASTFM_API_KEY"))
}

/// The fanart.tv project API key.
pub fn fanart() -> Option<&'static str> {
    built_in(option_env!("OPENTAG_FANARTTV_API_KEY"))
}

/// How Discogs requests authenticate: the user's personal token first, then
/// OpenTag's application key and secret, else anonymously.
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

/// The user's own key when set, else the built-in one.
pub fn choose(user: &str, built_in: Option<&'static str>) -> Option<String> {
    let user = user.trim();
    if user.is_empty() {
        built_in.map(str::to_owned)
    } else {
        Some(user.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::super::http::Auth;
    use super::*;

    #[test]
    fn empty_build_values_count_as_missing() {
        assert_eq!(built_in(None), None);
        assert_eq!(built_in(Some("  ")), None);
        assert_eq!(built_in(Some(" abc ")), Some("abc"));
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
    fn user_keys_replace_built_in_keys() {
        assert_eq!(choose(" user ", Some("built")).as_deref(), Some("user"));
        assert_eq!(choose("", Some("built")).as_deref(), Some("built"));
        assert_eq!(choose("", None), None);
    }
}
