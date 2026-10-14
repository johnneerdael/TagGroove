//! Android SAF document selection, retained grants and streamed document editing.
//! Calls block until the native result arrives: use `spawn_blocking` from async
//! Tauri commands. The Android picker itself never blocks the Android UI thread.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentSnapshot {
    pub document: DocumentInfo,
    pub sha256: String,
    pub bytes: u64,
}

#[cfg(any(target_os = "android", test))]
#[derive(Deserialize)]
struct StorageRootResponse {
    path: PathBuf,
}
#[cfg(any(target_os = "android", test))]
impl StorageRootResponse {
    fn into_path(self) -> Result<PathBuf, String> {
        if !self.path.is_absolute() {
            return Err("Native storage root must be absolute".into());
        }
        Ok(self.path)
    }
}

#[cfg(target_os = "android")]
#[derive(Serialize)]
struct StorageRootRequest {}

#[cfg(any(target_os = "android", test))]
#[derive(Deserialize)]
struct CreateResponse {
    document: Option<DocumentInfo>,
}
#[cfg(target_os = "android")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotRequest<'a> {
    uri: &'a str,
    output_path: &'a str,
}
#[cfg(target_os = "android")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublishRequest<'a> {
    uri: &'a str,
    input_path: &'a str,
    expected_sha256: &'a str,
}
#[cfg(target_os = "android")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateRequest<'a> {
    name: &'a str,
    mime_type: Option<&'a str>,
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
    /// Actual canonical Android Context.filesDir; resolve on a blocking worker.
    pub fn storage_root(&self) -> Result<PathBuf, String> {
        #[cfg(target_os = "android")]
        {
            self.0
                .run_mobile_plugin::<StorageRootResponse>("storageRoot", StorageRootRequest {})
                .map_err(|error| error.to_string())?
                .into_path()
        }
        #[cfg(not(target_os = "android"))]
        Err(unsupported())
    }
    /// Stream into a new private file. Failed snapshots retain any partial output.
    pub fn snapshot(&self, uri: &str, output_path: &Path) -> Result<DocumentSnapshot, String> {
        #[cfg(target_os = "android")]
        {
            validate_uri(uri)?;
            let output_path = path_text(output_path)?;
            self.0
                .run_mobile_plugin("snapshot", SnapshotRequest { uri, output_path })
                .map_err(|error| error.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (uri, output_path);
            Err(unsupported())
        }
    }
    /// Check current bytes, write with truncation, then verify a fresh provider read.
    /// SAF cannot guarantee atomic replacement or compare-and-swap isolation.
    pub fn publish(
        &self,
        uri: &str,
        input_path: &Path,
        expected_sha256: &str,
    ) -> Result<DocumentSnapshot, String> {
        #[cfg(target_os = "android")]
        {
            validate_uri(uri)?;
            validate_sha256(expected_sha256)?;
            let input_path = path_text(input_path)?;
            self.0
                .run_mobile_plugin(
                    "publish",
                    PublishRequest {
                        uri,
                        input_path,
                        expected_sha256,
                    },
                )
                .map_err(|error| error.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (uri, input_path, expected_sha256);
            Err(unsupported())
        }
    }
    pub fn create_document(
        &self,
        name: &str,
        mime_type: Option<&str>,
    ) -> Result<Option<DocumentInfo>, String> {
        #[cfg(target_os = "android")]
        {
            validate_name(name)?;
            self.0
                .run_mobile_plugin::<CreateResponse>(
                    "createDocument",
                    CreateRequest { name, mime_type },
                )
                .map(|response| response.document)
                .map_err(|error| error.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (name, mime_type);
            Err(unsupported())
        }
    }
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

#[cfg(target_os = "android")]
fn path_text(path: &Path) -> Result<&str, String> {
    let text = path
        .to_str()
        .ok_or("Private file path must be valid UTF-8")?;
    if !path.is_absolute() || text.contains('\0') {
        return Err("Expected an absolute private file path".into());
    }
    // Android validates against actual Context.filesDir/cacheDir and rejects links.
    Ok(text)
}
#[cfg(any(target_os = "android", test))]
fn validate_sha256(value: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Expected SHA-256 must contain exactly 64 hexadecimal characters".into());
    }
    Ok(())
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
    fn storage_root_keeps_native_absolute_path_and_rejects_relative_roots() {
        let path = std::env::temp_dir().canonicalize().unwrap();
        let response: StorageRootResponse =
            serde_json::from_value(serde_json::json!({"path":path})).unwrap();
        assert_eq!(response.into_path().unwrap(), path);
        let relative: StorageRootResponse = serde_json::from_str(r#"{"path":"files"}"#).unwrap();
        assert!(relative.into_path().is_err());
    }
    #[test]
    fn create_cancellation_is_nullable_and_snapshot_keeps_large_lengths() {
        let cancelled: CreateResponse = serde_json::from_str(r#"{"document":null}"#).unwrap();
        assert!(cancelled.document.is_none());
        let snapshot: DocumentSnapshot = serde_json::from_str(r#"{"document":{"uri":"content://provider/id","name":"a.wav","mime_type":null,"size":null,"modified_ms":null,"writable":true,"persisted":false},"sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","bytes":4294967296}"#).unwrap();
        assert_eq!(snapshot.bytes, 4_294_967_296);
        assert_eq!(snapshot.document.size, None);
    }
    #[test]
    fn expected_hash_is_exactly_a_sha256_hex_digest() {
        assert!(validate_sha256(&"aB".repeat(32)).is_ok());
        for invalid in ["", "abc", &"g".repeat(64), &"a".repeat(65)] {
            assert!(validate_sha256(invalid).is_err());
        }
    }
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
