use std::io::{self, Write};
use std::process::{Command, Output, Stdio};

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn assert_finding(output: &Output) -> Result<(), serde_json::Error> {
    let value: serde_json::Value = serde_json::from_str(&stdout(output))?;
    assert_eq!(
        value
            .pointer("/hookSpecificOutput/additionalContext")
            .and_then(serde_json::Value::as_str),
        Some("no-mut  src/a.rs:4  mutation in domain")
    );
    Ok(())
}

#[test]
fn hook_ignores_malformed_input_and_reports_matching_findings() -> io::Result<()> {
    let malformed = run_hook("not json", None)?;
    assert!(malformed.status.success());
    assert_eq!(stdout(&malformed), "");
    let fixture_dir = fixture("check-new");
    let found = run_hook(
        r#"{"tool_input":{"file_path":"src/a.rs"}}"#,
        Some(&fixture_dir),
    )?;
    assert!(found.status.success());
    assert_finding(&found).map_err(io::Error::other)?;
    let absolute = format!("{}/src/a.rs", env!("CARGO_MANIFEST_DIR"));
    let found = run_hook(
        &format!(r#"{{"tool_input":{{"file_path":"{absolute}"}}}}"#),
        Some(&fixture_dir),
    )?;
    assert!(found.status.success());
    assert_finding(&found).map_err(io::Error::other)?;
    Ok(())
}

#[test]
fn hook_prints_nothing_when_there_are_no_findings() -> io::Result<()> {
    let clear = run_hook(
        r#"{"tool_input":{"file_path":"src/a.rs"}}"#,
        Some(&fixture("check-pass")),
    )?;
    assert!(clear.status.success());
    assert_eq!(stdout(&clear), "");
    Ok(())
}

#[test]
fn hook_succeeds_when_checks_are_missing_or_sarif_fails() -> io::Result<()> {
    let missing = std::env::temp_dir().join(format!("charter-hook-missing-{}", std::process::id()));
    let absent = run_hook(
        r#"{"tool_input":{"file_path":"src/a.rs"}}"#,
        Some(&missing.display().to_string()),
    )?;
    assert!(absent.status.success());
    let failed = run_hook(
        r#"{"tool_input":{"file_path":"src/a.rs"}}"#,
        Some(&fixture("check-badsarif")),
    )?;
    assert!(failed.status.success());
    assert_eq!(stdout(&failed), "");
    assert!(String::from_utf8_lossy(&failed.stderr).contains("invalid SARIF"));
    Ok(())
}

fn run_hook(input: &str, dir: Option<&str>) -> io::Result<Output> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_charter"));
    command
        .arg("hook")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = dir {
        command.args(["--dir", dir]);
    }
    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("stdin unavailable"))?
        .write_all(input.as_bytes())?;
    child.wait_with_output()
}
