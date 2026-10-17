//! Wire companions for the existing action/column import boundaries.
use serde::Serialize;
use tag_core::batch_errors::ErrorCause;
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ImportFailure {
    Plain(String),
    Parts {
        message: String,
        import_failure: Box<ErrorCause>,
    },
}
impl ImportFailure {
    pub fn parts(cause: ErrorCause) -> Self {
        Self::Parts {
            message: cause.to_string(),
            import_failure: Box::new(cause),
        }
    }
}
impl std::fmt::Display for ImportFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plain(text) => f.write_str(text),
            Self::Parts { message, .. } => f.write_str(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn import_failure_preserves_plain_string_and_typed_invoke_message_association() {
        let plain = ImportFailure::Plain("Help: raw outer guard".into());
        let value = serde_json::to_value(&plain).unwrap();
        let ipc: tauri::ipc::InvokeError = plain.into();
        assert_eq!(value, serde_json::json!("Help: raw outer guard"));
        assert_eq!(ipc.0, value);
        let failure = ImportFailure::parts(ErrorCause::external("Action field must not be empty"));
        let value = serde_json::to_value(&failure).unwrap();
        let ipc: tauri::ipc::InvokeError = failure.clone().into();
        assert_eq!(ipc.0, value);
        assert_eq!(value["message"], failure.to_string());
        assert_eq!(value["import_failure"]["kind"], "external");
    }
}
