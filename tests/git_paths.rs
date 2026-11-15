#![cfg(unix)]

#[path = "support/git.rs"]
mod git;
#[path = "support/temp_dir.rs"]
mod temp_dir;
use std::{io, path::Path};

fn commit(repo: &Path) -> io::Result<()> {
    assert!(git::fixture_git(repo, &["add", "."])?.status.success());
    assert!(
        git::fixture_git(repo, &["commit", "-m", "fixture"])?
            .status
            .success()
    );
    Ok(())
}
fn fixture(repo: &Path, name: &str) -> io::Result<()> {
    std::fs::create_dir_all(repo.join(".charter"))?;
    for args in [
        &["init"][..],
        &["config", "user.name", "Test"],
        &["config", "user.email", "test@example.com"],
    ] {
        assert!(git::fixture_git(repo, args)?.status.success());
    }
    std::fs::write(
        repo.join(".charter/rules.toml"),
        "version=1\n[[rule]]\nid='C1'\nsection='Code'\ntext='rule'\nenforce=['check']",
    )?;
    std::fs::write(
        repo.join(".charter/checks.toml"),
        "[[check]]\nid='scan'\nrules=['C1']\nrun='cat findings.json'\noutput='sarif'",
    )?;
    let finding = serde_json::json!({"ruleId":"r", "message":{"text":"violation"}, "locations":[{"physicalLocation":{"artifactLocation":{"uri":name}}}]});
    std::fs::write(
        repo.join("findings.json"),
        serde_json::json!({"runs":[{"results":[finding]}]}).to_string(),
    )?;
    std::fs::write(repo.join(name), "old")?;
    commit(repo)
}
#[test]
fn committed_odd_filename_is_not_silently_filtered() -> io::Result<()> {
    let temp = temp_dir::TempDir::create()?;
    let repo = temp.0.as_path();
    let name = "-space quote'\t\n.rs";
    fixture(repo, name)?;
    std::fs::write(repo.join(name), "changed")?;
    commit(repo)?;
    let out = git::isolated_command(env!("CARGO_BIN_EXE_charter"))
        .current_dir(repo)
        .args(["check", "--changed", "HEAD^", "--json"])
        .output()?;
    assert_eq!(out.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(
        value["lines"],
        serde_json::json!([format!("scan  {name}:0  r: violation")])
    );
    Ok(())
}

#[test]
fn invalid_utf8_path_reports_failure_without_complete_json() -> io::Result<()> {
    let temp = temp_dir::TempDir::create()?;
    let repo = temp.0.as_path();
    fixture(repo, "valid.rs")?;
    commit_invalid_path(repo)?;
    let out = git::isolated_command(env!("CARGO_BIN_EXE_charter"))
        .current_dir(repo)
        .args(["check", "--changed", "HEAD^", "--json"])
        .output()?;
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let event: serde_json::Value = serde_json::from_str(stderr.lines().next().unwrap_or(""))?;
    assert_eq!(event["event"], "charter.git_paths.failed");
    assert_eq!(event["kind"], "invalid_utf8");
    assert_eq!(event["metric"], "charter_git_path_failures");
    assert_eq!(event["count"], 1);
    assert!(stderr.contains("git paths are not valid UTF-8"));
    Ok(())
}

fn commit_invalid_path(repo: &Path) -> io::Result<()> {
    use std::io::Write;
    use std::process::Stdio;
    let blob = git::fixture_git(repo, &["rev-parse", "HEAD:valid.rs"])?;
    let entry = format!("100644 {}\t", String::from_utf8_lossy(&blob.stdout).trim());
    let mut child = git::isolated_command("git")
        .current_dir(repo)
        .args(["update-index", "-z", "--index-info"])
        .stdin(Stdio::piped())
        .spawn()?;
    if let Some(mut input) = child.stdin.take() {
        input.write_all(&[entry.as_bytes(), &[255, 0]].concat())?;
    }
    assert!(child.wait()?.success());
    assert!(
        git::fixture_git(repo, &["commit", "-m", "invalid path"])?
            .status
            .success()
    );
    Ok(())
}
