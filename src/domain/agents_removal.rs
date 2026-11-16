use super::agents::{AgentsError, BEGIN, END, EditAction, FileEdit};

/// The edits that undo `agents_edits`: drop the charter block from AGENTS.md and
/// CLAUDE.md, and delete a file that held nothing else. Text outside the markers stays.
///
/// # Errors
///
/// `BrokenMarkers` when a file has a lone, repeated or reversed charter marker.
pub fn agents_removal_edits(
    agents_md: Option<&str>,
    claude_md: Option<&str>,
) -> Result<Vec<FileEdit>, AgentsError> {
    [("AGENTS.md", agents_md), ("CLAUDE.md", claude_md)]
        .into_iter()
        .map(|(path, text)| {
            let after =
                without_block(text.unwrap_or_default()).ok_or(AgentsError::BrokenMarkers(path))?;
            Ok(removal(path, text, after))
        })
        .collect()
}

fn removal(path: &'static str, before: Option<&str>, after: String) -> FileEdit {
    let action = match before {
        None => EditAction::Unchanged,
        Some(text) if text == after => EditAction::Unchanged,
        Some(_) if after.trim().is_empty() => EditAction::Delete,
        Some(_) => EditAction::Update,
    };
    FileEdit {
        path,
        action,
        contents: after,
    }
}

fn without_block(text: &str) -> Option<String> {
    match (text.matches(BEGIN).count(), text.matches(END).count()) {
        (0, 0) => Some(text.to_owned()),
        (1, 1) => {
            let (head, tail) = text.split_once(BEGIN)?;
            let (_, rest) = tail.split_once(END)?;
            let rest = rest.strip_prefix('\n').unwrap_or(rest);
            Some(match rest {
                "" => head.strip_suffix('\n').unwrap_or(head).to_owned(),
                _ => format!("{head}{rest}"),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::agents::agents_edits;
    use super::*;

    fn round_trip(agents: Option<&str>, claude: Option<&str>) -> Vec<FileEdit> {
        let installed = agents_edits(agents, claude).unwrap_or_default();
        agents_removal_edits(Some(&installed[0].contents), Some(&installed[1].contents))
            .unwrap_or_default()
    }

    #[test]
    fn uninstall_restores_existing_files_and_deletes_created_ones() {
        for original in ["# Repo\n", "# Repo\n\n", "# Repo\n\n\n"] {
            let removed = round_trip(Some(original), None);
            assert_eq!(removed[0].contents, original);
            assert_eq!(removed[1].action, EditAction::Delete);
        }
    }

    #[test]
    fn uninstall_keeps_text_on_both_sides_of_the_block() {
        let text = format!("top\n{BEGIN}\nx\n{END}\nbottom\n");
        let removed = agents_removal_edits(Some(&text), None).unwrap_or_default();
        assert_eq!(removed[0].contents, "top\nbottom\n");
        assert_eq!(removed[1].action, EditAction::Unchanged);
    }

    #[test]
    fn uninstall_leaves_an_import_charter_did_not_write() {
        let removed = agents_removal_edits(None, Some("@AGENTS.md\n")).unwrap_or_default();
        assert!(
            removed
                .iter()
                .all(|edit| edit.action == EditAction::Unchanged)
        );
    }

    #[test]
    fn uninstall_rejects_broken_markers() {
        assert_eq!(
            agents_removal_edits(None, Some(&format!("{END}\n{BEGIN}\n"))),
            Err(AgentsError::BrokenMarkers("CLAUDE.md"))
        );
    }
}
