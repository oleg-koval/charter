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
        "{\"ok\":false,\"gaps\":[{\"id\":\"C2\",\"file\":\"rules.toml\",\"message\":\"promises check, but no [[check]] lists it\"}],\"waived\":[]}\n"
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

#[test]
fn check_changed_filters_sarif_to_committed_changed_paths() -> io::Result<()> {
    let (repo, base) = changed_repo()?;
    let out = run_from(&repo, &["check", "--changed", &base, "--dir", ".charter"])?;
    assert!(out.status.success(), "{}", stdout(&out));
    assert_eq!(stdout(&out), "");
    let out = run_from(
        &repo,
        &["check", "--changed", "missing-base", "--dir", ".charter"],
    )?;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(stdout(&out), "");
    assert!(stderr(&out).contains("git diff --name-only missing-base...HEAD"));
    let out = run_from(&repo, &["check", "--dir", ".charter"])?;
    assert_eq!(out.status.code(), Some(1));
    assert!(stdout(&out).contains("src/a.rs:2"));
    std::fs::remove_dir_all(repo)?;
    Ok(())
}

fn changed_repo() -> io::Result<(std::path::PathBuf, String)> {
    let repo = std::env::temp_dir().join(format!("charter-changed-{}", std::process::id()));
    let config = repo.join(".charter");
    std::fs::create_dir_all(&config)?;
    write_changed_config(&repo, &config)?;
    init_git(&repo)?;
    let base = String::from_utf8_lossy(&git(&repo, &["rev-parse", "HEAD"])?.stdout)
        .trim()
        .to_owned();
    commit_unrelated_file(&repo)?;
    Ok((repo, base))
}
fn write_changed_config(repo: &std::path::Path, config: &std::path::Path) -> io::Result<()> {
    let sarif = repo.join("findings.sarif");
    std::fs::write(
        &sarif,
        r#"{"runs":[{"results":[{"ruleId":"r1","message":{"text":"bad"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"src/a.rs"},"region":{"startLine":2}}}]}]}]}"#,
    )?;
    std::fs::write(
        config.join("rules.toml"),
        "version=1\n[[rule]]\nid='C1'\nsection='Code'\ntext='rule'\nenforce=['check']\n",
    )?;
    std::fs::write(
        config.join("checks.toml"),
        format!(
            "[[check]]\nid='ast'\nrules=['C1']\nrun='cat {}'\noutput='sarif'\n",
            sarif.display()
        ),
    )?;
    Ok(())
}
fn init_git(repo: &std::path::Path) -> io::Result<()> {
    assert!(git(repo, &["init"])?.status.success());
    assert!(
        git(repo, &["config", "user.email", "test@example.com"])?
            .status
            .success()
    );
    assert!(
        git(repo, &["config", "user.name", "Test"])?
            .status
            .success()
    );
    assert!(git(repo, &["add", "."])?.status.success());
    assert!(git(repo, &["commit", "-m", "base"])?.status.success());
    Ok(())
}
fn commit_unrelated_file(repo: &std::path::Path) -> io::Result<()> {
    std::fs::create_dir_all(repo.join("src"))?;
    std::fs::write(repo.join("src/other.rs"), "changed")?;
    assert!(git(repo, &["add", "."])?.status.success());
    assert!(
        git(repo, &["commit", "-m", "change unrelated file"])?
            .status
            .success()
    );
    Ok(())
}

fn run_from(repo: &std::path::Path, args: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_charter"))
        .current_dir(repo)
        .args(args)
        .output()
}

fn git(repo: &std::path::Path, args: &[&str]) -> io::Result<Output> {
    Command::new("git").current_dir(repo).args(args).output()
}
