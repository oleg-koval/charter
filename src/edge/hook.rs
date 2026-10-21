use std::io::{self, Read, Write};
use std::path::Path;
use std::process::ExitCode;

use super::{emit, read};
use crate::domain::{self, Finding, Outcome, Output};

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

fn run_hook(dir: &Path, input: &mut impl Read, out: &mut impl Write, err: &mut impl Write) {
    let mut text = String::new();
    if let Err(error) = input.read_to_string(&mut text) {
        let _ = emit(err, &error.to_string(), ExitCode::SUCCESS);
        return;
    }
    let Some(file) = domain::edited_file(&text) else {
        return;
    };
    let root =
        std::env::current_dir().map_or_else(|_| String::new(), |path| path.display().to_string());
    match hook_findings(dir, &file, &root) {
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

fn hook_findings(dir: &Path, file: &str, root: &str) -> Result<Vec<Finding>, String> {
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
                .flat_map(|outcome| matching_findings(outcome, file, root))
                .collect()
        })
}

fn matching_findings(outcome: &Outcome, file: &str, root: &str) -> Vec<Finding> {
    match outcome {
        Outcome::Sarif { findings, .. } => findings
            .iter()
            .filter(|finding| domain::same_path(root, &finding.file, file))
            .cloned()
            .collect(),
        Outcome::Exit { .. } => Vec::new(),
    }
}
