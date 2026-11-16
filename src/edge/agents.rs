use std::fs;
use std::io;
use std::path::Path;
use std::process::ExitCode;

use super::emit;
use crate::domain::{self, EditAction, FileEdit};

#[must_use]
pub fn agents_install(root: &Path, dry_run: bool) -> ExitCode {
    match install(root, dry_run) {
        Ok(text) => emit(&mut io::stdout().lock(), &text, ExitCode::SUCCESS),
        Err(message) => emit(&mut io::stderr().lock(), &message, ExitCode::FAILURE),
    }
}

fn install(root: &Path, dry_run: bool) -> Result<String, String> {
    let agents = read_optional(&root.join("AGENTS.md"))?;
    let claude = read_optional(&root.join("CLAUDE.md"))?;
    let edits = domain::agents_edits(agents.as_deref(), claude.as_deref())
        .map_err(|error| error.to_string())?;
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
    fs::write(&path, &edit.contents).map_err(|error| format!("{}: {error}", path.display()))
}
