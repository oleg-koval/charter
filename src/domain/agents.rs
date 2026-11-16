use std::fmt;

const BEGIN: &str = "<!-- charter:begin -->";
const END: &str = "<!-- charter:end -->";
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
}

impl fmt::Display for AgentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BrokenMarkers(path) => write!(
                f,
                "{path}: `{BEGIN}` and `{END}` must both appear once, in that order; fix them by hand"
            ),
        }
    }
}

/// The edits that put charter's block in AGENTS.md and import AGENTS.md from CLAUDE.md.
/// Text outside the charter markers is kept byte for byte.
///
/// # Errors
///
/// `BrokenMarkers` when AGENTS.md has a lone, repeated or reversed charter marker.
pub fn agents_edits(
    agents_md: Option<&str>,
    claude_md: Option<&str>,
) -> Result<Vec<FileEdit>, AgentsError> {
    let agents =
        with_block(agents_md.unwrap_or_default()).ok_or(AgentsError::BrokenMarkers("AGENTS.md"))?;
    let claude = with_import(claude_md.unwrap_or_default());
    Ok(vec![
        edit("AGENTS.md", agents_md, agents),
        edit("CLAUDE.md", claude_md, claude),
    ])
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

fn with_block(text: &str) -> Option<String> {
    let block = format!("{BEGIN}\n{BLOCK}{END}\n");
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

fn with_import(text: &str) -> String {
    if text.lines().any(|line| line.trim() == IMPORT) {
        text.to_owned()
    } else {
        format!("{text}{}{IMPORT}\n", separator(text))
    }
}

fn separator(text: &str) -> &'static str {
    match (
        text.is_empty(),
        text.ends_with("\n\n"),
        text.ends_with('\n'),
    ) {
        (true, _, _) | (_, true, _) => "",
        (_, _, true) => "\n",
        _ => "\n\n",
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
        assert_eq!(created[1].contents, "@AGENTS.md\n");
    }

    #[test]
    fn appends_after_existing_text_and_keeps_it() {
        let updated = edits(Some("# Repo\nOwn rules."), Some("# Claude\n"));
        assert!(
            updated[0]
                .contents
                .starts_with("# Repo\nOwn rules.\n\n<!-- charter:begin -->")
        );
        assert_eq!(updated[1].contents, "# Claude\n\n@AGENTS.md\n");
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
}
