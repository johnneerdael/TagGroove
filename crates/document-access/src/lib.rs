//! Android SAF document selection, retained URI grants, metadata and renames.
//! Calls block until the native result arrives: use `spawn_blocking` from async
//! Tauri commands. The Android picker itself never blocks the Android UI thread.

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentInfo {
    pub uri: String,
    /// Provider display name, not a filesystem path or inferred URI component.
    pub name: String,
    pub mime_type: Option<String>,
    pub size: Option<u64>,
    pub modified_ms: Option<i64>,
    /// Both a current write grant and provider byte-write support are present.
    pub writable: bool,
    /// Read access is retained; when writable, write access is retained too.
    pub persisted: bool,
}

#[cfg(target_os = "android")]
pub struct Documents<R: Runtime>(tauri::plugin::PluginHandle<R>);
#[cfg(not(target_os = "android"))]
pub struct Documents<R: Runtime>(std::marker::PhantomData<fn() -> R>);

#[cfg(target_os = "android")]
#[derive(Deserialize)]
struct PickResponse {
    documents: Vec<DocumentInfo>,
}
#[cfg(target_os = "android")]
#[derive(Serialize)]
struct PickRequest {
    multiple: bool,
}
#[cfg(target_os = "android")]
#[derive(Serialize)]
struct UriRequest<'a> {
    uri: &'a str,
}
#[cfg(target_os = "android")]
#[derive(Serialize)]
struct RenameRequest<'a> {
    uri: &'a str,
    name: &'a str,
}

impl<R: Runtime> Documents<R> {
    pub fn pick_documents(&self, multiple: bool) -> Result<Vec<DocumentInfo>, String> {
        #[cfg(target_os = "android")]
        {
            self.0
                .run_mobile_plugin::<PickResponse>("pickDocuments", PickRequest { multiple })
                .map(|response| response.documents)
                .map_err(|error| error.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = multiple;
            Err(unsupported())
        }
    }
    pub fn info(&self, uri: &str) -> Result<DocumentInfo, String> {
        #[cfg(target_os = "android")]
        {
            validate_uri(uri)?;
            self.0
                .run_mobile_plugin("info", UriRequest { uri })
                .map_err(|error| error.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = uri;
            Err(unsupported())
        }
    }
    pub fn rename(&self, uri: &str, name: &str) -> Result<DocumentInfo, String> {
        #[cfg(target_os = "android")]
        {
            validate_uri(uri)?;
            validate_name(name)?;
            self.0
                .run_mobile_plugin("rename", RenameRequest { uri, name })
                .map_err(|error| error.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (uri, name);
            Err(unsupported())
        }
    }
}

#[cfg(not(target_os = "android"))]
fn unsupported() -> String {
    "Android SAF document access is not supported on this platform".into()
}

#[cfg(any(target_os = "android", test))]
fn validate_uri(uri: &str) -> Result<(), String> {
    if !uri.starts_with("content://")
        || uri["content://".len()..]
            .split('/')
            .next()
            .unwrap_or_default()
            .is_empty()
        || uri.contains('\0')
    {
        return Err("Expected an Android content document URI".into());
    }
    Ok(())
}
#[cfg(any(target_os = "android", test))]
fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() || matches!(name, "." | "..") || name.contains(['/', '\\', '\0']) {
        return Err("Document name must be one nonempty filename without path separators".into());
    }
    Ok(())
}

pub trait DocumentsExt<R: Runtime> {
    fn documents(&self) -> &Documents<R>;
}
impl<R: Runtime, T: Manager<R>> DocumentsExt<R> for T {
    fn documents(&self) -> &Documents<R> {
        self.state::<Documents<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("document-access")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let documents = Documents(
                api.register_android_plugin("org.opentag.documentaccess", "DocumentAccessPlugin")?,
            );
            #[cfg(not(target_os = "android"))]
            let documents = {
                let _ = api;
                Documents::<R>(std::marker::PhantomData)
            };
            app.manage(documents);
            Ok(())
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_keeps_unknown_values_and_readonly_capability() {
        let info:DocumentInfo=serde_json::from_str(r#"{"uri":"content://provider/document/1","name":"track.flac","mime_type":null,"size":null,"modified_ms":null,"writable":false,"persisted":true}"#).unwrap();
        assert_eq!(info.size, None);
        assert_eq!(info.modified_ms, None);
        assert!(!info.writable);
        assert!(info.persisted);
    }
    #[test]
    fn uri_identity_is_opaque_and_only_content_scheme_is_accepted() {
        assert!(validate_uri("content://provider/document/primary%3AMusic%2Ftrack.flac").is_ok());
        for uri in [
            "/music/track.flac",
            "file:///music/track.flac",
            "content:///document/1",
            "https://provider/document/1",
        ] {
            assert!(validate_uri(uri).is_err());
        }
    }
    #[test]
    fn renames_cannot_be_paths_but_preserve_valid_unicode_names() {
        for name in [
            "",
            " ",
            ".",
            "..",
            "folder/track.flac",
            "folder\\track.flac",
            "track\0.flac",
        ] {
            assert!(validate_name(name).is_err());
        }
        assert!(validate_name("Björk — Jóga.flac").is_ok());
    }
}
