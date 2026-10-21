use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use super::emit;

const RULES: &str = include_str!("../../.charter/rules.toml");
const CHECKS: &str = r#"# Replace the placeholder check with the repo's real tools.
[[check]]
id = "starter"
rules = ["C1", "C2", "C3", "C4", "C5", "C7", "C10", "C23"]
run = "true"
output = "exit"

[[artifact]]
heading = "Problem"
rules = ["C11"]
[[artifact]]
heading = "Data Structures"
rules = ["C12"]
[[artifact]]
heading = "Reuse"
rules = ["C16"]
[[artifact]]
heading = "Alternatives"
rules = ["C17"]
[[artifact]]
heading = "Tradeoffs"
rules = ["C18"]
[[artifact]]
heading = "Performance"
rules = ["C28"]
[[artifact]]
heading = "Latency"
rules = ["C29"]
[[artifact]]
heading = "Scope"
rules = ["C30"]
[[artifact]]
heading = "CI"
rules = ["C32"]
[[artifact]]
heading = "TL;DR (for humans)"
rules = ["C39"]
first = true
"#;

#[must_use]
pub fn init(dir: &Path) -> ExitCode {
    match initialize(dir) {
        Ok(()) => emit(
            &mut io::stdout().lock(),
            &format!("initialized {}", dir.display()),
            ExitCode::SUCCESS,
        ),
        Err(message) => emit(&mut io::stderr().lock(), &message, ExitCode::FAILURE),
    }
}

fn initialize(dir: &Path) -> Result<(), String> {
    let rules = dir.join("rules.toml");
    let checks = dir.join("checks.toml");
    fs::create_dir_all(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    write_new(&rules, RULES)?;
    if let Err(error) = write_new(&checks, CHECKS) {
        fs::remove_file(&rules)
            .map_err(|cleanup| format!("{error}; {}: {cleanup}", rules.display()))?;
        return Err(error);
    }
    Ok(())
}

fn write_new(path: &Path, contents: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                format!("{} already exists", path.display())
            } else {
                format!("{}: {error}", path.display())
            }
        })?;
    file.write_all(contents.as_bytes())
        .map_err(|error| format!("{}: {error}", path.display()))
}
