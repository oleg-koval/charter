use std::fs;
use std::io;
use std::path::Path;
use std::process::ExitCode;

use super::emit;
use crate::domain::{self, AgentsError, EditAction, FileEdit};

type Plan = fn(Option<&str>, Option<&str>) -> Result<Vec<FileEdit>, AgentsError>;
type Steps = [(Plan, [&'static str; 2]); 2];

const INSTRUCTIONS: [&str; 2] = ["AGENTS.md", "CLAUDE.md"];
const HOOKS: [&str; 2] = [".claude/settings.json", ".codex/hooks.json"];

#[must_use]
pub fn agents_install(root: &Path, dry_run: bool) -> ExitCode {
    let steps: Steps = [
        (domain::agents_edits, INSTRUCTIONS),
        (domain::hook_edits, HOOKS),
    ];
    run(root, dry_run, &steps)
}

#[must_use]
pub fn agents_uninstall(root: &Path, dry_run: bool) -> ExitCode {
    let steps: Steps = [
        (domain::agents_removal_edits, INSTRUCTIONS),
        (domain::hook_removal_edits, HOOKS),
    ];
    run(root, dry_run, &steps)
}

fn run(root: &Path, dry_run: bool, steps: &Steps) -> ExitCode {
    match apply(root, dry_run, steps) {
        Ok(text) => emit(&mut io::stdout().lock(), &text, ExitCode::SUCCESS),
        Err(message) => emit(&mut io::stderr().lock(), &message, ExitCode::FAILURE),
    }
}

// Every step is planned before anything is written, so a bad file aborts with no writes.
fn apply(root: &Path, dry_run: bool, steps: &Steps) -> Result<String, String> {
    let edits = steps
        .iter()
        .map(|(plan, [first, second])| {
            let first = read_optional(&root.join(first))?;
            let second = read_optional(&root.join(second))?;
            plan(first.as_deref(), second.as_deref()).map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?
        .concat();
    if !dry_run {
        edits
            .iter()
            .filter(|edit| edit.action != EditAction::Unchanged)
            .try_for_each(|edit| write(root, edit))?;
    }
    Ok(domain::render_edits(&edits, dry_run))
}

fn read_optional(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn write(root: &Path, edit: &FileEdit) -> Result<(), String> {
    let path = root.join(edit.path);
    let written = match edit.action {
        EditAction::Delete => fs::remove_file(&path),
        _ => path
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(&path, &edit.contents)),
    };
    written.map_err(|error| format!("{}: {error}", path.display()))
}
