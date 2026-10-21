use std::io;
use std::process::{Command, Output};

fn charter(args: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_charter"))
        .args(args)
        .output()
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn doctor_passes_on_the_repos_own_charter() -> io::Result<()> {
    let dir = format!("{}/.charter", env!("CARGO_MANIFEST_DIR"));
    let out = charter(&["doctor", "--dir", &dir])?;
    assert!(out.status.success());
    assert_eq!(stdout(&out), "ok  40 rules, 7 checks, 9 artifacts\n");
    Ok(())
}

#[test]
fn doctor_reports_an_unmapped_layer_as_one_line() -> io::Result<()> {
    let out = charter(&["doctor", "--dir", &fixture("unmapped")])?;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        stdout(&out),
        "C2  rules.toml  promises check, but no [[check]] lists it\n"
    );
    Ok(())
}

#[test]
fn doctor_json_lists_the_gaps() -> io::Result<()> {
    let out = charter(&["doctor", "--json", "--dir", &fixture("unmapped")])?;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        stdout(&out),
        "{\"ok\":false,\"gaps\":[{\"id\":\"C2\",\"file\":\"rules.toml\",\"message\":\"promises check, but no [[check]] lists it\"}]}\n"
    );
    Ok(())
}

#[test]
fn doctor_fails_on_a_missing_dir_without_panicking() -> io::Result<()> {
    let out = charter(&["doctor", "--dir", &fixture("does-not-exist")])?;
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("rules.toml"));
    assert!(!stderr(&out).contains("panicked"));
    Ok(())
}
