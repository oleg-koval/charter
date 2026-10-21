use std::io::{self, Read, Write};
use std::path::Path;
use std::process::ExitCode;

use super::{emit, read};
use crate::domain::{self, Outcome, Output};

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
    match hook_findings(dir, &file) {
        Ok(lines) => lines.iter().for_each(|line| {
            let _ = emit(out, line, ExitCode::SUCCESS);
        }),
        Err(message) => {
            let _ = emit(err, &message, ExitCode::SUCCESS);
        }
    }
}

fn hook_findings(dir: &Path, file: &str) -> Result<Vec<String>, String> {
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
                .flat_map(|outcome| matching_lines(outcome, file))
                .collect()
        })
}

fn matching_lines(outcome: &Outcome, file: &str) -> Vec<String> {
    match outcome {
        Outcome::Sarif { findings, .. } => findings
            .iter()
            .filter(|finding| finding.file == file)
            .map(|finding| {
                format!(
                    "{}  {}:{}  {}",
                    finding.rule_id, finding.file, finding.line, finding.message
                )
            })
            .collect(),
        Outcome::Exit { .. } => Vec::new(),
    }
}
