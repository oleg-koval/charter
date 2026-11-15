use std::fmt;

#[derive(Debug)]
pub enum GitPathsError {
    Spawn(String),
    Exit(String),
    InvalidUtf8,
}

impl fmt::Display for GitPathsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(message) | Self::Exit(message) => f.write_str(message),
            Self::InvalidUtf8 => f.write_str("git paths are not valid UTF-8"),
        }
    }
}

#[must_use]
pub fn invalid_path_event() -> String {
    serde_json::json!({"event":"charter.git_paths.failed", "kind":"invalid_utf8", "metric":"charter_git_path_failures", "count":1}).to_string()
}
