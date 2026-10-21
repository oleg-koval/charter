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
#[test]
fn check_reports_a_new_sarif_finding() -> io::Result<()> {
    let out = charter(&["check", "--dir", &fixture("check-new")])?;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        stdout(&out),
        "ast  src/a.rs:4  no-mut: mutation in domain\n"
    );
    Ok(())
}

#[test]
fn check_passes_a_baselined_sarif_finding() -> io::Result<()> {
    let out = charter(&["check", "--dir", &fixture("check-baselined")])?;
    assert!(out.status.success());
    assert_eq!(stdout(&out), "");
    Ok(())
}

#[test]
fn baseline_accepts_current_findings_so_check_passes() -> io::Result<()> {
    let src = fixture("check-mixed");
    let dir = std::env::temp_dir().join(format!("charter-baseline-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    for name in ["rules.toml", "checks.toml"] {
        std::fs::copy(format!("{src}/{name}"), dir.join(name))?;
    }
    let dir_arg = dir.display().to_string();
    let before = charter(&["check", "--dir", &dir_arg])?;
    let written = charter(&["baseline", "--dir", &dir_arg])?;
    let after = charter(&["check", "--dir", &dir_arg])?;
    let json = std::fs::read_to_string(dir.join("baseline.json"))?;
    std::fs::remove_dir_all(&dir)?;
    assert_eq!(before.status.code(), Some(1));
    assert!(written.status.success(), "{}", stderr(&written));
    assert!(json.contains("\"fp1\""), "{json}");
    assert_eq!(stdout(&after), "lint  exited 1\n");
    Ok(())
}

#[test]
fn check_rejects_output_that_is_not_sarif() -> io::Result<()> {
    let out = charter(&["check", "--dir", &fixture("check-badsarif")])?;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout(&out), "");
    assert!(stderr(&out).contains("invalid SARIF"), "{}", stderr(&out));
    Ok(())
}

#[test]
fn check_rejects_a_baseline_of_the_wrong_shape() -> io::Result<()> {
    let out = charter(&["check", "--dir", &fixture("check-badbaseline")])?;
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("invalid baseline"),
        "{}",
        stderr(&out)
    );
    Ok(())
}

#[test]
fn rubric_prints_review_rules_and_json_is_encoded() -> io::Result<()> {
    let dir = format!("{}/.charter", env!("CARGO_MANIFEST_DIR"));
    let out = charter(&["rubric", "--dir", &dir])?;
    assert!(out.status.success());
    assert!(stdout(&out).contains("- C1 | Code | NEVER mutate"));
    let json = charter(&["rubric", "--json", "--dir", &dir])?;
    assert!(json.status.success());
    assert!(serde_json::from_slice::<String>(&json.stdout).is_ok());
    Ok(())
}

#[test]
fn verify_review_reports_missing_file_and_malformed_table() -> io::Result<()> {
    let dir = format!("{}/.charter", env!("CARGO_MANIFEST_DIR"));
    let missing = fixture("missing-review.md");
    let out = charter(&["verify-review", &missing, "--dir", &dir])?;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout(&out), "");
    assert!(stderr(&out).contains("missing-review.md"));
    let malformed = fixture("bad-review.md");
    let out = charter(&["verify-review", &malformed, "--dir", &dir])?;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout(&out), "");
    assert!(stderr(&out).contains("three columns"));
    Ok(())
}

#[test]
fn verify_pr_reports_a_missing_body_file() -> io::Result<()> {
    let dir = format!("{}/.charter", env!("CARGO_MANIFEST_DIR"));
    let missing = fixture("missing-pr-body.md");
    let out = charter(&["verify-pr", &missing, "--dir", &dir])?;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout(&out), "");
    assert!(stderr(&out).contains("missing-pr-body.md"));
    Ok(())
}
