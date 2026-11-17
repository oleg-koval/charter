use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use super::emit;
use crate::domain::{self, AgentsError, EditAction, FileEdit};

type Plan = fn(Option<&str>, Option<&str>) -> Result<Vec<FileEdit>, AgentsError>;
type Steps = [(Plan, [&'static str; 2]); 2];

const INSTRUCTIONS: [&str; 2] = domain::INSTRUCTION_PATHS;
const HOOKS: [&str; 2] = domain::HOOK_PATHS;

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
            let edits = plan(
                read_optional(&root.join(first))?.as_deref(),
                read_optional(&root.join(second))?.as_deref(),
            )
            .map_err(|error| error.to_string())?;
            Ok::<_, String>(covered_by_first(root, [first, second], edits))
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

// A symlinked pair (CLAUDE.md -> AGENTS.md) is one file: writing both edits would make each run
// undo the last, so the second path is left to the first file's edit.
fn covered_by_first(root: &Path, paths: [&str; 2], edits: Vec<FileEdit>) -> Vec<FileEdit> {
    let [first, second] = paths.map(|path| identity(&root.join(path)));
    if first.is_none() || first != second {
        return edits;
    }
    edits
        .into_iter()
        .map(|edit| {
            let action = if edit.path == paths[1] {
                EditAction::Unchanged
            } else {
                edit.action
            };
            FileEdit { action, ..edit }
        })
        .collect()
}

// Which file a path names, even when it is a symlink to a file that does not exist yet.
fn identity(path: &Path) -> Option<PathBuf> {
    if let Ok(real) = fs::canonicalize(path) {
        return Some(real);
    }
    let target = fs::read_link(path).map_or_else(
        |_| path.to_path_buf(),
        |link| path.parent().map_or(link.clone(), |dir| dir.join(&link)),
    );
    Some(
        fs::canonicalize(target.parent()?)
            .ok()?
            .join(target.file_name()?),
    )
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
