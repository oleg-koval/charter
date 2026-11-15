use std::fmt;

#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum ImportError {
    Read(String),
    Policy(String),
    Parse(String),
    Mapping(String),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Read(message)
            | Self::Policy(message)
            | Self::Parse(message)
            | Self::Mapping(message) => message,
        };
        f.write_str(message)
    }
}

#[must_use]
pub fn import_failure(error: &ImportError) -> String {
    serde_json::json!({"event": "charter.import.failed", "metric": "charter_import_failures", "count": 1, "error": error}).to_string()
}
