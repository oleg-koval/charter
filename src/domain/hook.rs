use serde_json::Value;

use super::Finding;

#[must_use]
pub fn edited_file(input: &str) -> Option<String> {
    serde_json::from_str::<Value>(input)
        .ok()?
        .get("tool_input")?
        .get("file_path")?
        .as_str()
        .map(str::to_owned)
}

#[must_use]
pub fn render_hook_output(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return String::new();
    }
    let additional_context = findings
        .iter()
        .map(|finding| {
            format!(
                "{}  {}:{}  {}",
                finding.rule_id, finding.file, finding.line, finding.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PostToolUse",
            "additionalContext": additional_context
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(message: &str) -> Finding {
        Finding {
            rule_id: "no-mut".to_owned(),
            file: "src/a.rs".to_owned(),
            line: 4,
            message: message.to_owned(),
            fingerprint: "fingerprint".to_owned(),
        }
    }

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

    #[test]
    fn renders_empty_findings_as_empty_output() {
        assert_eq!(render_hook_output(&[]), "");
    }

    #[test]
    fn renders_findings_as_post_tool_use_context() {
        let output = render_hook_output(&[finding("mutation in domain")]);
        let value: Value = serde_json::from_str(&output).expect("valid JSON");
        assert_eq!(
            value.pointer("/hookSpecificOutput/additionalContext"),
            Some(&Value::String(
                "no-mut  src/a.rs:4  mutation in domain".to_owned()
            ))
        );
        assert_eq!(
            value.pointer("/hookSpecificOutput/hookEventName"),
            Some(&Value::String("PostToolUse".to_owned()))
        );
    }

    #[test]
    fn escapes_quotes_and_newlines_in_finding_messages() {
        let message = "quoted \"message\"\nsecond line";
        let value: Value =
            serde_json::from_str(&render_hook_output(&[finding(message)])).expect("valid JSON");
        assert_eq!(
            value
                .pointer("/hookSpecificOutput/additionalContext")
                .and_then(Value::as_str),
            Some("no-mut  src/a.rs:4  quoted \"message\"\nsecond line")
        );
    }
}
