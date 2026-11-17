use std::fmt;

/// The instruction files, in the order every planner takes and returns them.
pub const INSTRUCTION_PATHS: [&str; 2] = ["AGENTS.md", "CLAUDE.md"];
const AGENTS: &str = INSTRUCTION_PATHS[0];
const CLAUDE: &str = INSTRUCTION_PATHS[1];

pub(super) const BEGIN: &str = "<!-- charter:begin -->";
pub(super) const END: &str = "<!-- charter:end -->";
const IMPORT: &str = "@AGENTS.md";

const BLOCK: &str = "## Charter

This repository's engineering contract is enforced by charter. The rules live in
`.charter/rules.toml`; how each one is proven lives in `.charter/checks.toml`.

- Before a commit, `charter check --working-tree` prints nothing and exits 0.
- `charter doctor` passes: every rule has the check, review or PR section it promises.
- Independent review: `charter rubric` prints the rules to rule on; the verdict table
  must pass `charter verify-review <file>`.
- A pull request body must pass `charter verify-pr <body-file>`.
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditAction {
    Create,
    Update,
    Delete,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEdit {
    pub path: &'static str,
    pub action: EditAction,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentsError {
    BrokenMarkers(&'static str),
    NotHookConfig(&'static str),
}

impl fmt::Display for AgentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BrokenMarkers(path) => write!(
                f,
                "{path}: `{BEGIN}` and `{END}` must both appear once, in that order; fix them by hand"
            ),
            Self::NotHookConfig(path) => write!(f, "{path}: not a hook config; fix it by hand"),
        }
    }
}

/// The edits that put charter's block in AGENTS.md and import AGENTS.md from CLAUDE.md.
/// Both go between charter markers so an uninstall removes exactly them; text outside the
/// markers is kept byte for byte, and a CLAUDE.md that already imports AGENTS.md is left alone.
///
/// # Errors
///
/// `BrokenMarkers` when a file has a lone, repeated or reversed charter marker.
pub fn agents_edits(
    agents_md: Option<&str>,
    claude_md: Option<&str>,
) -> Result<Vec<FileEdit>, AgentsError> {
    let agents = with_block(agents_md.unwrap_or_default(), BLOCK)
        .ok_or(AgentsError::BrokenMarkers(AGENTS))?;
    let claude =
        with_import(claude_md.unwrap_or_default()).ok_or(AgentsError::BrokenMarkers(CLAUDE))?;
    Ok(vec![
        edit(AGENTS, agents_md, agents),
        edit(CLAUDE, claude_md, claude),
    ])
}

#[must_use]
pub fn render_edits(edits: &[FileEdit], dry_run: bool) -> String {
    edits
        .iter()
        .map(|edit| {
            let (prefix, verb) = match (edit.action, dry_run) {
                (EditAction::Create, true) => ("would ", "create"),
                (EditAction::Update, true) => ("would ", "update"),
                (EditAction::Create, false) => ("", "create"),
                (EditAction::Update, false) => ("", "update"),
                (EditAction::Delete, true) => ("would ", "delete"),
                (EditAction::Delete, false) => ("", "delete"),
                (EditAction::Unchanged, _) => ("", "unchanged"),
            };
            format!("{prefix}{verb}  {}", edit.path)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn edit(path: &'static str, before: Option<&str>, after: String) -> FileEdit {
    let action = match before {
        None => EditAction::Create,
        Some(text) if text == after => EditAction::Unchanged,
        Some(_) => EditAction::Update,
    };
    FileEdit {
        path,
        action,
        contents: after,
    }
}

fn with_block(text: &str, body: &str) -> Option<String> {
    let block = format!("{BEGIN}\n{body}{END}\n");
    match (text.matches(BEGIN).count(), text.matches(END).count()) {
        (0, 0) => Some(format!("{text}{}{block}", separator(text))),
        (1, 1) => {
            let (head, tail) = text.split_once(BEGIN)?;
            let (_, rest) = tail.split_once(END)?;
            Some(format!(
                "{head}{block}{}",
                rest.strip_prefix('\n').unwrap_or(rest)
            ))
        }
        _ => None,
    }
}

fn with_import(text: &str) -> Option<String> {
    let imported = text.lines().any(|line| line.trim() == IMPORT);
    if imported && !text.contains(BEGIN) {
        Some(text.to_owned())
    } else {
        with_block(text, &format!("{IMPORT}\n"))
    }
}

// Always exactly one blank line before an appended block, so uninstall can drop exactly
// one newline and give back the original bytes.
fn separator(text: &str) -> &'static str {
    match (text.is_empty(), text.ends_with('\n')) {
        (true, _) => "",
        (false, true) => "\n",
        (false, false) => "\n\n",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edits(agents: Option<&str>, claude: Option<&str>) -> Vec<FileEdit> {
        agents_edits(agents, claude).unwrap_or_default()
    }

    #[test]
    fn creates_both_files_when_missing() {
        let created = edits(None, None);
        assert_eq!(created[0].action, EditAction::Create);
        assert!(created[0].contents.starts_with(BEGIN));
        assert_eq!(created[1].contents, format!("{BEGIN}\n@AGENTS.md\n{END}\n"));
    }

    #[test]
    fn appends_after_existing_text_and_keeps_it() {
        let updated = edits(Some("# Repo\nOwn rules."), Some("# Claude\n"));
        assert!(
            updated[0]
                .contents
                .starts_with("# Repo\nOwn rules.\n\n<!-- charter:begin -->")
        );
        assert_eq!(
            updated[1].contents,
            format!("# Claude\n\n{BEGIN}\n@AGENTS.md\n{END}\n")
        );
        assert!(updated.iter().all(|edit| edit.action == EditAction::Update));
    }

    #[test]
    fn rerunning_changes_nothing() {
        let first = edits(Some("# Repo\n"), None);
        let again = edits(Some(&first[0].contents), Some(&first[1].contents));
        assert!(
            again
                .iter()
                .all(|edit| edit.action == EditAction::Unchanged)
        );
    }

    #[test]
    fn refreshes_a_stale_block_without_touching_text_around_it() {
        let stale = format!("top\n{BEGIN}\nold\n{END}\nbottom\n");
        let refreshed = &edits(Some(&stale), None)[0].contents;
        assert!(refreshed.starts_with(&format!("top\n{BEGIN}\n## Charter")));
        assert!(refreshed.ends_with(&format!("{END}\nbottom\n")));
        assert!(!refreshed.contains("old"));
    }

    #[test]
    fn rejects_broken_or_repeated_markers() {
        for text in [
            format!("{BEGIN}\n"),
            format!("{END}\n{BEGIN}\n"),
            format!("{BEGIN}{END}{BEGIN}{END}"),
        ] {
            assert_eq!(
                agents_edits(Some(&text), None),
                Err(AgentsError::BrokenMarkers("AGENTS.md"))
            );
        }
    }

    #[test]
    fn keeps_an_existing_import() {
        let kept = edits(None, Some("intro\n  @AGENTS.md  \nmore\n"));
        assert_eq!(kept[1].action, EditAction::Unchanged);
    }

    #[test]
    fn renders_each_edit_on_its_own_line() {
        let created = edits(None, None);
        assert_eq!(
            render_edits(&created, false),
            "create  AGENTS.md\ncreate  CLAUDE.md"
        );
        assert_eq!(
            render_edits(&created, true),
            "would create  AGENTS.md\nwould create  CLAUDE.md"
        );
    }
}
