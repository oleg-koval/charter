use serde_json::Value;

#[must_use]
pub fn edited_file(input: &str) -> Option<String> {
    serde_json::from_str::<Value>(input)
        .ok()?
        .get("tool_input")?
        .get("file_path")?
        .as_str()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_tool_input_file_path() {
        assert_eq!(
            edited_file(r#"{"tool_input":{"file_path":"src/a.rs"}}"#),
            Some("src/a.rs".to_owned())
        );
    }

    #[test]
    fn ignores_malformed_or_missing_paths() {
        assert_eq!(edited_file("not json"), None);
        assert_eq!(edited_file(r#"{"tool_input":{}}"#), None);
        assert_eq!(edited_file(r#"{"tool_input":{"file_path":2}}"#), None);
    }
}
