use serde_json::{Map, Value};

use super::agents::{AgentsError, FileEdit};
use super::agents_hooks::{Target, hooks_of, is_charter_hook, plan, runs_charter, with_key};

/// The edits that remove every hook running `charter hook`, dropping entries, the
/// `PostToolUse` list and `hooks` once they are empty, and deleting a file left empty.
///
/// # Errors
///
/// `NotHookConfig`, as for `hook_edits`.
pub fn hook_removal_edits(
    claude_settings: Option<&str>,
    codex_hooks: Option<&str>,
) -> Result<Vec<FileEdit>, AgentsError> {
    plan(claude_settings, codex_hooks, removed)
}

fn removed(target: &Target, root: &Map<String, Value>) -> Result<Value, AgentsError> {
    let (hooks, post) = hooks_of(root, target)?;
    if !runs_charter(root) {
        return Ok(Value::Object(root.clone()));
    }
    let kept: Vec<Value> = post.iter().filter_map(without_charter).collect();
    let post = (!kept.is_empty()).then_some(Value::Array(kept));
    let hooks = with_key(&hooks, "PostToolUse", post);
    let hooks = (!hooks.is_empty()).then_some(Value::Object(hooks));
    Ok(Value::Object(with_key(root, "hooks", hooks)))
}

fn is_charters(entry: &Value) -> bool {
    entry
        .get("hooks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(is_charter_hook)
}

fn without_charter(entry: &Value) -> Option<Value> {
    let Some(object) = entry.as_object().filter(|_| is_charters(entry)) else {
        return Some(entry.clone());
    };
    let kept: Vec<Value> = entry
        .get("hooks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|hook| !is_charter_hook(hook))
        .cloned()
        .collect();
    (!kept.is_empty()).then(|| Value::Object(with_key(object, "hooks", Some(Value::Array(kept)))))
}

#[cfg(test)]
mod tests {
    use super::super::agents::EditAction;
    use super::super::agents_hooks::hook_edits;
    use super::*;

    const OTHER: &str = r#"{"model":"x","hooks":{"PostToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"lint"}]}]}}"#;

    fn parsed(edit: &FileEdit) -> Value {
        serde_json::from_str(&edit.contents).unwrap_or_default()
    }

    #[test]
    fn removal_keeps_other_hooks_and_deletes_files_left_empty() {
        let added = hook_edits(Some(OTHER), None).unwrap_or_default();
        let removed = hook_removal_edits(Some(&added[0].contents), Some(&added[1].contents))
            .unwrap_or_default();
        assert_eq!(removed[0].action, EditAction::Update);
        assert_eq!(
            parsed(&removed[0]),
            serde_json::from_str::<Value>(OTHER).unwrap_or_default()
        );
        assert_eq!(removed[1].action, EditAction::Delete);
    }

    #[test]
    fn removal_leaves_configs_without_charter_byte_for_byte() {
        let kept = hook_removal_edits(Some(OTHER), None).unwrap_or_default();
        assert_eq!(kept[0].action, EditAction::Unchanged);
        assert_eq!(kept[0].contents, OTHER);
        assert_eq!(kept[1].action, EditAction::Unchanged);
    }
}
