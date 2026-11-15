use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use super::emit;
use crate::domain::{self, ImportError, InitOptionsError};

const RULES: &str = include_str!("../../.charter/rules.toml");
const CHECKS: &str = r#"# Replace the placeholder check with the repo's real tools.
[[check]]
id = "starter"
rules = ["C1", "C2", "C3", "C4", "C5", "C7", "C10", "C23"]
run = "echo 'Configure real checks in checks.toml before running charter check.' >&2; exit 1"
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
    init_with_checks(dir, None)
}

#[must_use]
pub fn init_with_checks(dir: &Path, source: Option<&Path>) -> ExitCode {
    init_with_options(dir, source, None)
}

#[must_use]
pub fn init_with_options(dir: &Path, source: Option<&Path>, recipe: Option<&str>) -> ExitCode {
    init_with_detection(dir, source, recipe, false)
}

#[must_use]
pub fn init_with_detection(
    dir: &Path,
    source: Option<&Path>,
    recipe: Option<&str>,
    detect: bool,
) -> ExitCode {
    let result = selected_checks(source, recipe, detect).map_err(|error| {
        emit(
            &mut io::stderr().lock(),
            &domain::init_options_failure(&error),
            ExitCode::FAILURE,
        );
        error.to_string()
    });
    match result.and_then(|checks| initialize(dir, &checks)) {
        Ok(()) => emit(
            &mut io::stdout().lock(),
            &format!("initialized {}", dir.display()),
            ExitCode::SUCCESS,
        ),
        Err(message) => emit(&mut io::stderr().lock(), &message, ExitCode::FAILURE),
    }
}

fn selected_checks(
    source: Option<&Path>,
    recipe: Option<&str>,
    detect: bool,
) -> Result<String, InitOptionsError> {
    match (detect, recipe) {
        (true, _) => detected_checks(),
        (false, Some(name)) => {
            let text = domain::recipe_checks(name, source.is_some())?;
            validate_checks(name, text.to_owned()).map_err(InitOptionsError::Import)
        }
        (false, None) => source
            .map_or_else(|| Ok(CHECKS.to_owned()), read_checks)
            .map_err(InitOptionsError::Import),
    }
}

fn detected_checks() -> Result<String, InitOptionsError> {
    let names = fs::read_dir(".")
        .map_err(|error| {
            InitOptionsError::ProjectRootRead(format!("cannot inspect current directory: {error}"))
        })?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            InitOptionsError::ProjectRootRead(format!(
                "cannot read current directory entry: {error}"
            ))
        })?;
    domain::detect_recipes(&names)
}

fn read_checks(source: &Path) -> Result<String, ImportError> {
    let (path, text) = super::read(source).map_err(ImportError::Read)?;
    validate_checks(&path, text)
}

fn validate_checks(path: &str, text: String) -> Result<String, ImportError> {
    let policy =
        domain::parse_policy(RULES).map_err(|error| ImportError::Policy(error.to_string()))?;
    let mechanism = domain::parse_mechanism(&text)
        .map_err(|error| ImportError::Parse(format!("{path}: {error}")))?;
    let gaps = domain::diagnose(&policy, &mechanism);
    if gaps.is_empty() {
        Ok(text)
    } else {
        Err(ImportError::Mapping(format!(
            "{path}:\n{}",
            gaps.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        )))
    }
}

fn initialize(dir: &Path, contents: &str) -> Result<(), String> {
    let rules = dir.join("rules.toml");
    let checks = dir.join("checks.toml");
    fs::create_dir_all(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    write_new(&rules, RULES)?;
    if let Err(error) = write_new(&checks, contents) {
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
