use std::fs;
use std::io;
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
    refuse_existing(&rules)?;
    refuse_existing(&checks)?;
    fs::create_dir_all(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    write(&rules, RULES)?;
    write(&checks, CHECKS)
}

fn refuse_existing(path: &Path) -> Result<(), String> {
    if path.exists() {
        Err(format!("{} already exists", path.display()))
    } else {
        Ok(())
    }
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}
