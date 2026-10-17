//! Companion presentation data for the eight batch-error frames. Legacy
//! FileError/ScanResult/Replay values and their English strings stay unchanged.
use crate::{operations::RenameFailure, FileError};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ErrorCause {
    /// A rendered message from a proven source-owned diagnostic factory.
    /// Unlike external IO/library text, its audited catalog roles may translate.
    OwnedText {
        value: String,
    },
    Owned {
        key: String,
        #[serde(default)]
        params: BTreeMap<String, String>,
        #[serde(default)]
        children: BTreeMap<String, ErrorCause>,
    },
    External {
        value: String,
    },
    Joined {
        parts: Vec<ErrorCause>,
        separator: String,
    },
    Rename {
        value: Box<RenameFailure>,
    },
}
impl ErrorCause {
    pub fn owned_text(value: impl Into<String>) -> Self {
        Self::OwnedText {
            value: value.into(),
        }
    }
    pub fn owned(key: impl Into<String>) -> Self {
        Self::Owned {
            key: key.into(),
            params: BTreeMap::new(),
            children: BTreeMap::new(),
        }
    }
    pub fn message<const N: usize>(key: impl Into<String>, params: [(&str, String); N]) -> Self {
        Self::Owned {
            key: key.into(),
            params: params
                .into_iter()
                .map(|(name, value)| (name.into(), value))
                .collect(),
            children: BTreeMap::new(),
        }
    }
    pub fn wrap<const N: usize>(
        key: impl Into<String>,
        params: [(&str, String); N],
        name: &str,
        cause: ErrorCause,
    ) -> Self {
        Self::Owned {
            key: key.into(),
            params: params
                .into_iter()
                .map(|(name, value)| (name.into(), value))
                .collect(),
            children: BTreeMap::from([(name.into(), cause)]),
        }
    }
    pub fn children<const N: usize>(
        key: impl Into<String>,
        children: [(&str, ErrorCause); N],
    ) -> Self {
        Self::Owned {
            key: key.into(),
            params: BTreeMap::new(),
            children: children
                .into_iter()
                .map(|(name, value)| (name.into(), value))
                .collect(),
        }
    }
    pub fn joined(parts: Vec<ErrorCause>, separator: impl Into<String>) -> Self {
        Self::Joined {
            parts,
            separator: separator.into(),
        }
    }
    pub fn external(error: impl fmt::Display) -> Self {
        Self::External {
            value: error.to_string(),
        }
    }
}
impl fmt::Display for ErrorCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owned {
                key,
                params,
                children,
            } => {
                // Replace source placeholders once: braces in raw values are data.
                static FIELD: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
                    regex::Regex::new(r"\{([A-Za-z][A-Za-z0-9]*)\}").unwrap()
                });
                let mut from = 0;
                for field in FIELD.captures_iter(key) {
                    let span = field.get(0).unwrap();
                    f.write_str(&key[from..span.start()])?;
                    if let Some(child) = children.get(&field[1]) {
                        write!(f, "{child}")?;
                    } else if let Some(value) = params.get(&field[1]) {
                        f.write_str(value)?;
                    } else {
                        f.write_str(span.as_str())?;
                    }
                    from = span.end();
                }
                f.write_str(&key[from..])
            }
            Self::OwnedText { value } | Self::External { value } => f.write_str(value),
            Self::Rename { value } => value.fmt(f),
            Self::Joined { parts, separator } => {
                for (index, part) in parts.iter().enumerate() {
                    if index > 0 {
                        f.write_str(separator)?;
                    }
                    write!(f, "{part}")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Undo,
    Redo,
}
impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Undo => "undone",
            Self::Redo => "redone",
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BatchError {
    Action {
        group: String,
        index: usize,
        cause: ErrorCause,
    },
    ActionValidation {
        group: String,
        cause: ErrorCause,
    },
    HistoryChanged {
        label: String,
        direction: Direction,
    },
    HistoryFailed {
        direction: Direction,
        cause: ErrorCause,
    },
    HistoryFoldersFailed {
        direction: Direction,
        cause: ErrorCause,
    },
    DestinationCollision {
        path: String,
    },
    PlaylistLine {
        line: usize,
        cause: ErrorCause,
    },
    SourceCoverFailed {
        cause: ErrorCause,
    },
}
impl fmt::Display for BatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Action {
                group,
                index,
                cause,
            } => write!(f, "{group} / action {index}: {cause}"),
            Self::ActionValidation { group, cause } => write!(f, "{group}: {cause}"),
            Self::HistoryChanged { label, direction } => write!(
                f,
                "Changed outside TagGroove after {label}; it was not {direction}"
            ),
            Self::HistoryFailed { direction, cause } => {
                write!(f, "It was not {direction}: {cause}")
            }
            Self::HistoryFoldersFailed { direction, cause } => write!(
                f,
                "Folders and side files could not be {direction}: {cause}"
            ),
            Self::DestinationCollision { path } => {
                write!(f, "Destination exists or another file maps to {path}")
            }
            Self::PlaylistLine { line, cause } => write!(f, "Line {line}: {cause}"),
            Self::SourceCoverFailed { cause } => {
                write!(f, "The cover image could not be saved: {cause}")
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexedDiagnostic {
    pub index: usize,
    pub diagnostic: BatchError,
}

/// Indexed presentation companions for an importer's unchanged reason strings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexedCause {
    pub index: usize,
    pub cause: ErrorCause,
}

/// An additive wire/report companion. Its result is still the legacy value.
#[derive(Debug, Serialize)]
pub struct Detailed<T> {
    #[serde(flatten)]
    pub result: T,
    pub error_diagnostics: Vec<IndexedDiagnostic>,
}
impl<T> Detailed<T> {
    pub fn plain(result: T) -> Self {
        Self {
            result,
            error_diagnostics: Vec::new(),
        }
    }
}
pub fn push_error(
    errors: &mut Vec<FileError>,
    diagnostics: &mut Vec<IndexedDiagnostic>,
    path: String,
    diagnostic: BatchError,
) {
    let error = diagnostic.to_string();
    diagnostics.push(IndexedDiagnostic {
        index: errors.len(),
        diagnostic,
    });
    errors.push(FileError { path, error });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn companion_frames_keep_legacy_english_and_add_only_indexed_metadata() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/batch-error-diagnostics.json"
        ))
        .unwrap();
        let mut result = crate::ScanResult::default();
        result.errors.push(FileError {
            path: "Help".into(),
            error: "Legacy error".into(),
        });
        let mut diagnostics = Vec::new();
        for case in cases.as_array().unwrap() {
            let diagnostic: BatchError =
                serde_json::from_value(case["diagnostic"].clone()).unwrap();
            assert_eq!(diagnostic.to_string(), case["source"].as_str().unwrap());
            push_error(
                &mut result.errors,
                &mut diagnostics,
                "Help: Title; Path".into(),
                diagnostic,
            );
        }
        let legacy = serde_json::to_value(&result).unwrap();
        let detailed = serde_json::to_value(Detailed {
            result,
            error_diagnostics: diagnostics,
        })
        .unwrap();
        assert_eq!(detailed["entries"], legacy["entries"]);
        assert_eq!(detailed["errors"], legacy["errors"]);
        assert_eq!(detailed["error_diagnostics"][0]["index"], 1);
        assert_eq!(
            detailed["error_diagnostics"].as_array().unwrap().len(),
            cases.as_array().unwrap().len()
        );
        assert_eq!(
            serde_json::to_value(Detailed::plain(crate::ScanResult::default())).unwrap()
                ["error_diagnostics"],
            serde_json::json!([])
        );
    }
}
