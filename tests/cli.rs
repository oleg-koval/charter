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
    let line = stdout(&out);
    assert!(line.starts_with("ok  "), "{line}");
    assert!(line.ends_with(" artifacts\n"), "{line}");
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

#[test]
fn doctor_reports_an_unparsable_file_with_its_reason() -> io::Result<()> {
    let out = charter(&["doctor", "--dir", &fixture("bad-layer")])?;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout(&out), "");
    assert!(stderr(&out).contains("unknown variant"), "{}", stderr(&out));
    Ok(())
}

#[test]
fn check_is_quiet_when_every_check_passes() -> io::Result<()> {
    let out = charter(&["check", "--dir", &fixture("check-pass")])?;
    assert!(out.status.success());
    assert_eq!(stdout(&out), "");
    Ok(())
}

#[test]
fn check_reports_a_failing_exit_check() -> io::Result<()> {
    let out = charter(&["check", "--dir", &fixture("check-fail")])?;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout(&out), "lint  exited 1\n");
    Ok(())
}

#[test]
fn check_json_lists_the_lines() -> io::Result<()> {
    let out = charter(&["check", "--json", "--dir", &fixture("check-fail")])?;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        stdout(&out),
        "{\"ok\":false,\"lines\":[\"lint  exited 1\"]}\n"
    );
    Ok(())
}

#[test]
fn check_refuses_to_run_when_doctor_has_gaps() -> io::Result<()> {
    let out = charter(&["check", "--dir", &fixture("unmapped")])?;
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stdout(&out).starts_with("C2  rules.toml"),
        "{}",
        stdout(&out)
    );
    Ok(())
}
