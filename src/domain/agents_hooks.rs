use serde_json::{Map, Value, json};

use super::agents::{AgentsError, EditAction, FileEdit};

// A command containing this marks an entry as charter's own, so uninstall removes only it.
const MARK: &str = "charter hook";

pub(super) struct Target {
    path: &'static str,
    matcher: &'static str,
    command: &'static str,
}

/// The hook config files, in the order `hook_edits` and `hook_removal_edits` take them.
pub const HOOK_PATHS: [&str; 2] = [TARGETS[0].path, TARGETS[1].path];

const TARGETS: [Target; 2] = [
    Target {
        path: ".claude/settings.json",
        matcher: "Edit|Write|MultiEdit",
        command: "cd \"$CLAUDE_PROJECT_DIR\" && charter hook",
    },
    Target {
        path: ".codex/hooks.json",
        matcher: "Edit|Write",
        command: "cd \"$(git rev-parse --show-toplevel)\" && charter hook",
    },
];

/// Adds charter's advisory `PostToolUse` hook to the Claude Code and Codex hook configs,
/// unless one already runs `charter hook`.
///
/// # Errors
///
/// `NotHookConfig` when a file is not a hook config JSON object.
pub fn hook_edits(claude: Option<&str>, codex: Option<&str>) -> Result<Vec<FileEdit>, AgentsError> {
    plan(claude, codex, added)
}

pub(super) type Change = fn(&Target, &Map<String, Value>) -> Result<Value, AgentsError>;

pub(super) fn plan(
    claude: Option<&str>,
    codex: Option<&str>,
    change: Change,
) -> Result<Vec<FileEdit>, AgentsError> {
    TARGETS
        .iter()
        .zip([claude, codex])
        .map(|(target, text)| {
            let root = parse(target, text.unwrap_or_default())?;
            let after = change(target, &root)?;
            Ok(file_edit(target.path, text, &Value::Object(root), &after))
        })
        .collect()
}

fn parse(target: &Target, text: &str) -> Result<Map<String, Value>, AgentsError> {
    match serde_json::from_str(text) {
        Ok(Value::Object(root)) => Ok(root),
        _ if text.trim().is_empty() => Ok(Map::new()),
        _ => Err(AgentsError::NotHookConfig(target.path)),
    }
}

fn file_edit(path: &'static str, text: Option<&str>, before: &Value, after: &Value) -> FileEdit {
    let empty = after.as_object().is_some_and(Map::is_empty);
    let (action, contents) = match (text, before == after, empty) {
        (Some(text), true, _) => (EditAction::Unchanged, text.to_owned()),
        (None, true, _) => (EditAction::Unchanged, String::new()),
        (Some(_), false, true) => (EditAction::Delete, String::new()),
        (None, false, _) => (EditAction::Create, pretty(after)),
        (Some(_), false, false) => (EditAction::Update, pretty(after)),
    };
    FileEdit {
        path,
        action,
        contents,
    }
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default() + "\n"
}

pub(super) fn runs_charter(root: &Map<String, Value>) -> bool {
    root.get("hooks")
        .and_then(|hooks| hooks.get("PostToolUse"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("hooks").and_then(Value::as_array))
        .flatten()
        .any(is_charter_hook)
}

pub(super) fn is_charter_hook(hook: &Value) -> bool {
    hook.get("command")
        .and_then(Value::as_str)
        .is_some_and(|command| command.contains(MARK))
}

pub(super) type Hooks = (Map<String, Value>, Vec<Value>);

pub(super) fn hooks_of(root: &Map<String, Value>, target: &Target) -> Result<Hooks, AgentsError> {
    let bad = AgentsError::NotHookConfig(target.path);
    let hooks = match root.get("hooks") {
        None => Map::new(),
        Some(Value::Object(hooks)) => hooks.clone(),
        Some(_) => return Err(bad),
    };
    match hooks.get("PostToolUse") {
        None => Ok((hooks, Vec::new())),
        Some(Value::Array(post)) => Ok((hooks.clone(), post.clone())),
        Some(_) => Err(bad),
    }
}

fn added(target: &Target, root: &Map<String, Value>) -> Result<Value, AgentsError> {
    let (hooks, post) = hooks_of(root, target)?;
    if runs_charter(root) {
        return Ok(Value::Object(root.clone()));
    }
    let entry = json!({
        "matcher": target.matcher,
        "hooks": [{ "type": "command", "command": target.command, "timeout": 120 }]
    });
    let post = post.into_iter().chain([entry]).collect();
    let hooks = Value::Object(with_key(&hooks, "PostToolUse", Some(Value::Array(post))));
    Ok(Value::Object(with_key(root, "hooks", Some(hooks))))
}

pub(super) fn with_key(
    map: &Map<String, Value>,
    key: &str,
    value: Option<Value>,
) -> Map<String, Value> {
    map.iter()
        .filter(|(name, _)| name.as_str() != key)
        .map(|(name, value)| (name.clone(), value.clone()))
        .chain(value.map(|value| (key.to_owned(), value)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const OTHER: &str = r#"{"model":"x","hooks":{"PostToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"lint"}]}]}}"#;

    fn at(edit: &FileEdit, pointer: &str) -> Value {
        let value: Value = serde_json::from_str(&edit.contents).unwrap_or_default();
        value.pointer(pointer).cloned().unwrap_or_default()
    }

    #[test]
    fn creates_both_configs_with_the_harness_specific_command() {
        let created = hook_edits(None, None).unwrap_or_default();
        assert!(created.iter().all(|edit| edit.action == EditAction::Create));
        assert_eq!(
            at(&created[0], "/hooks/PostToolUse/0/matcher"),
            "Edit|Write|MultiEdit"
        );
        assert_eq!(
            at(&created[1], "/hooks/PostToolUse/0/hooks/0/command"),
            TARGETS[1].command
        );
    }

    #[test]
    fn appends_beside_other_hooks_and_settings_and_rerun_is_unchanged() {
        let added = hook_edits(Some(OTHER), None).unwrap_or_default();
        assert_eq!(at(&added[0], "/model"), "x");
        assert_eq!(
            at(&added[0], "/hooks/PostToolUse/0/hooks/0/command"),
            "lint"
        );
        assert_eq!(
            at(&added[0], "/hooks/PostToolUse/1/matcher"),
            "Edit|Write|MultiEdit"
        );
        let again =
            hook_edits(Some(&added[0].contents), Some(&added[1].contents)).unwrap_or_default();
        assert!(again.iter().all(|e| e.action == EditAction::Unchanged));
    }

    #[test]
    fn rejects_files_that_are_not_hook_configs() {
        for text in [
            "not json",
            "[]",
            r#"{"hooks":[]}"#,
            r#"{"hooks":{"PostToolUse":{}}}"#,
        ] {
            assert_eq!(
                hook_edits(Some(text), None),
                Err(AgentsError::NotHookConfig(".claude/settings.json"))
            );
        }
    }
}
