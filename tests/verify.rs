use std::io;
use std::process::{Command, Output};

fn charter(args: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_charter"))
        .args(args)
        .output()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn setup(name: &str) -> io::Result<std::path::PathBuf> {
    let dir = std::env::temp_dir().join(format!("charter-verify-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("rules.toml"),
        "version=1\n[[rule]]\nid='R1'\nsection='Review'\ntext='review'\nenforce=['review']\n[[rule]]\nid='P1'\nsection='Artifact'\ntext='artifact'\nenforce=['artifact']\n",
    )?;
    std::fs::write(
        dir.join("checks.toml"),
        "[[artifact]]\nheading='Problem'\nrules=['P1']\n",
    )?;
    Ok(dir)
}

#[test]
fn verify_review_reports_failures_and_supports_json() -> io::Result<()> {
    let dir = setup("review")?;
    let review = dir.join("review.md");
    std::fs::write(
        &review,
        "| rule | verdict | evidence |\n|---|---|---|\n| R1 | FAIL | src/lib.rs:1 |\n",
    )?;
    let dir_arg = dir.display().to_string();
    let file = review.display().to_string();
    let failed = charter(&["verify-review", &file, "--dir", &dir_arg])?;
    assert_eq!(failed.status.code(), Some(1));
    assert_eq!(stdout(&failed), "R1  FAIL\n");
    let json = charter(&["verify-review", &file, "--dir", &dir_arg, "--json"])?;
    assert_eq!(stdout(&json), "{\"ok\":false,\"lines\":[\"R1  FAIL\"]}\n");
    std::fs::write(
        &review,
        "| rule | verdict | evidence |\n|---|---|---|\n| R1 | PASS | src/lib.rs:1 |\n",
    )?;
    let valid = charter(&["verify-review", &file, "--dir", &dir_arg])?;
    assert!(valid.status.success());
    assert_eq!(stdout(&valid), "");
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

#[test]
fn verify_pr_reports_failures_and_accepts_a_valid_body() -> io::Result<()> {
    let dir = setup("pr")?;
    let body = dir.join("body.md");
    std::fs::write(&body, "## Other\nText\n")?;
    let dir_arg = dir.display().to_string();
    let file = body.display().to_string();
    let failed = charter(&["verify-pr", &file, "--dir", &dir_arg])?;
    assert_eq!(failed.status.code(), Some(1));
    assert_eq!(stdout(&failed), "Problem  artifact heading is missing\n");
    let json = charter(&["verify-pr", &file, "--dir", &dir_arg, "--json"])?;
    assert_eq!(
        stdout(&json),
        "{\"ok\":false,\"lines\":[\"Problem  artifact heading is missing\"]}\n"
    );
    std::fs::write(&body, "## Problem\nDetails\n")?;
    let valid = charter(&["verify-pr", &file, "--dir", &dir_arg])?;
    assert!(valid.status.success());
    assert_eq!(stdout(&valid), "");
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

#[test]
fn verification_bad_inputs_fail_on_stderr() -> io::Result<()> {
    let review = charter(&[
        "verify-review",
        "tests/fixtures/bad-review.md",
        "--dir",
        "tests/fixtures/check-pass",
    ])?;
    assert_eq!(review.status.code(), Some(1));
    assert!(stderr(&review).contains("three columns"));
    let pr = charter(&[
        "verify-pr",
        "missing-body.md",
        "--dir",
        "tests/fixtures/check-pass",
    ])?;
    assert_eq!(pr.status.code(), Some(1));
    assert!(stderr(&pr).contains("missing-body.md"));
    Ok(())
}
