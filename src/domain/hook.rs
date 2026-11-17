use std::path::Path;

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

// Codex reports an apply_patch edit as patch text in `tool_input.command`, not as a
// `file_path`. Deleted files are left out: there is nothing left to check.
const PATCH_TARGETS: [&str; 3] = ["*** Update File: ", "*** Add File: ", "*** Move to: "];

/// Files an edit touched: Claude Code's `tool_input.file_path`, or every target in a
/// Codex `apply_patch` body.
#[must_use]
pub fn edited_files(input: &str) -> Vec<String> {
    hook_payload_files(input).unwrap_or_default()
}

/// Like `edited_files`, but `None` when the input is not a hook payload at all (not JSON,
/// or no `tool_input` object), so the caller can say so instead of staying silent.
/// Relative targets are joined onto the payload's `cwd`: Codex patch paths are relative to
/// the session directory, not to wherever the hook runs.
#[must_use]
pub fn hook_payload_files(input: &str) -> Option<Vec<String>> {
    let value = serde_json::from_str::<Value>(input).ok()?;
    value.get("tool_input")?.as_object()?;
    let field = |name: &str| value.pointer(&format!("/tool_input/{name}"))?.as_str();
    let files = match (field("file_path"), field("command")) {
        (Some(path), _) => vec![path.to_owned()],
        (None, Some(patch)) => patch_targets(patch),
        (None, None) => Vec::new(),
    };
    let cwd = value.get("cwd").and_then(Value::as_str);
    Some(files.iter().map(|file| in_session(cwd, file)).collect())
}

fn in_session(cwd: Option<&str>, file: &str) -> String {
    match cwd {
        Some(cwd) if Path::new(file).is_relative() => {
            Path::new(cwd).join(file).display().to_string()
        }
        _ => file.to_owned(),
    }
}

fn patch_targets(patch: &str) -> Vec<String> {
    patch
        .lines()
        .filter_map(|line| {
            PATCH_TARGETS
                .iter()
                .find_map(|prefix| line.strip_prefix(prefix))
        })
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
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
    fn reads_claude_file_path_as_one_edited_file() {
        assert_eq!(
            edited_files(r#"{"tool_input":{"file_path":"src/a.rs"}}"#),
            vec!["src/a.rs".to_owned()]
        );
    }

    #[test]
    fn reads_every_live_target_of_a_codex_patch() {
        let patch = "*** Begin Patch\n*** Update File: src/a.rs\n@@\n-x\n+y\n*** Delete File: src/gone.rs\n*** Add File: src/b.rs\n+z\n*** Update File: src/old.rs\n*** Move to: src/new.rs\n*** End Patch";
        let input = serde_json::json!({
            "tool_name": "apply_patch",
            "tool_input": { "command": patch }
        })
        .to_string();
        assert_eq!(
            edited_files(&input),
            ["src/a.rs", "src/b.rs", "src/old.rs", "src/new.rs"].map(str::to_owned)
        );
    }

    #[test]
    fn joins_relative_codex_targets_onto_the_session_cwd() {
        let input = serde_json::json!({
            "cwd": "/r/crates/foo",
            "tool_input": { "command": "*** Begin Patch\n*** Update File: src/lib.rs\n*** End Patch" }
        })
        .to_string();
        assert_eq!(
            hook_payload_files(&input),
            Some(vec!["/r/crates/foo/src/lib.rs".to_owned()])
        );
        let absolute = r#"{"cwd":"/r","tool_input":{"file_path":"/r/src/a.rs"}}"#;
        assert_eq!(
            hook_payload_files(absolute),
            Some(vec!["/r/src/a.rs".to_owned()])
        );
    }

    #[test]
    fn tells_a_non_payload_from_a_payload_without_targets() {
        assert_eq!(hook_payload_files("not json"), None);
        assert_eq!(hook_payload_files(r#"{"other":1}"#), None);
        assert_eq!(
            hook_payload_files(r#"{"tool_input":{"command":"ls"}}"#),
            Some(Vec::new())
        );
    }

    #[test]
    fn finds_no_edited_files_in_malformed_or_unrelated_input() {
        assert!(edited_files("not json").is_empty());
        assert!(edited_files(r#"{"tool_input":{}}"#).is_empty());
        assert!(edited_files(r#"{"tool_input":{"command":"ls -la"}}"#).is_empty());
        assert!(edited_files(r#"{"tool_input":{"command":"*** Add File: "}}"#).is_empty());
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
