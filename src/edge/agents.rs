use std::fs;
use std::io;
use std::path::Path;
use std::process::ExitCode;

use super::emit;
use crate::domain::{self, AgentsError, EditAction, FileEdit};

type Plan = fn(Option<&str>, Option<&str>) -> Result<Vec<FileEdit>, AgentsError>;

#[must_use]
pub fn agents_install(root: &Path, dry_run: bool) -> ExitCode {
    run(root, dry_run, domain::agents_edits)
}

#[must_use]
pub fn agents_uninstall(root: &Path, dry_run: bool) -> ExitCode {
    run(root, dry_run, domain::agents_removal_edits)
}

fn run(root: &Path, dry_run: bool, plan: Plan) -> ExitCode {
    match apply(root, dry_run, plan) {
        Ok(text) => emit(&mut io::stdout().lock(), &text, ExitCode::SUCCESS),
        Err(message) => emit(&mut io::stderr().lock(), &message, ExitCode::FAILURE),
    }
}

fn apply(root: &Path, dry_run: bool, plan: Plan) -> Result<String, String> {
    let agents = read_optional(&root.join("AGENTS.md"))?;
    let claude = read_optional(&root.join("CLAUDE.md"))?;
    let edits = plan(agents.as_deref(), claude.as_deref()).map_err(|error| error.to_string())?;
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
        _ => fs::write(&path, &edit.contents),
    };
    written.map_err(|error| format!("{}: {error}", path.display()))
}
