use std::io::{self, Read, Write};
use std::path::Path;
use std::process::ExitCode;

use super::{emit, read};
use crate::domain::{self, Finding, Outcome, Output};

const NOT_A_PAYLOAD: &str = "charter hook: input is not a PostToolUse payload (JSON with a tool_input object); nothing checked";

#[must_use]
pub fn hook(dir: &Path) -> ExitCode {
    run_hook(
        dir,
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    ExitCode::SUCCESS
}

fn read_payload(input: &mut impl Read) -> Result<Vec<String>, String> {
    let mut text = String::new();
    input
        .read_to_string(&mut text)
        .map_err(|error| error.to_string())?;
    domain::hook_payload_files(&text)
        .map(|edited| edited.iter().map(|file| resolve(file)).collect())
        .ok_or_else(|| NOT_A_PAYLOAD.to_owned())
}

fn run_hook(dir: &Path, input: &mut impl Read, out: &mut impl Write, err: &mut impl Write) {
    let files = match read_payload(input) {
        Ok(files) => files,
        Err(message) => {
            let _ = emit(err, &message, ExitCode::SUCCESS);
            return;
        }
    };
    if files.is_empty() {
        return;
    }
    let root =
        std::env::current_dir().map_or_else(|_| String::new(), |path| path.display().to_string());
    match hook_findings(dir, &files, &root) {
        Ok(findings) => {
            let output = domain::render_hook_output(&findings);
            if !output.is_empty() {
                let _ = emit(out, &output, ExitCode::SUCCESS);
            }
        }
        Err(message) => {
            let _ = emit(err, &message, ExitCode::SUCCESS);
        }
    }
}

// getcwd returns the resolved path, but the editor may report the file through a symlink
// (macOS /tmp, /var), so the root prefix would never match.
fn resolve(file: &str) -> String {
    let path = Path::new(file);
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) if path.is_absolute() => parent.canonicalize().map_or_else(
            |_| file.to_owned(),
            |real| real.join(name).display().to_string(),
        ),
        _ => file.to_owned(),
    }
}

fn hook_findings(dir: &Path, files: &[String], root: &str) -> Result<Vec<Finding>, String> {
    let (_, config) = read(&dir.join("checks.toml"))?;
    let mechanism = domain::parse_mechanism(&config).map_err(|error| error.to_string())?;
    mechanism
        .check
        .iter()
        .filter(|check| check.output == Output::Sarif)
        .map(super::check::execute)
        .collect::<Result<Vec<_>, _>>()
        .map(|outcomes| {
            outcomes
                .iter()
                .flat_map(|outcome| matching_findings(outcome, files, root))
                .collect()
        })
}

fn matching_findings(outcome: &Outcome, files: &[String], root: &str) -> Vec<Finding> {
    match outcome {
        Outcome::Sarif { findings, .. } => findings
            .iter()
            .filter(|finding| {
                files
                    .iter()
                    .any(|file| domain::same_path(root, &finding.file, file))
            })
            .cloned()
            .collect(),
        Outcome::Exit { .. } => Vec::new(),
    }
}
