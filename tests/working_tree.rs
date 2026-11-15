#![cfg(unix)]
#[path = "support/git.rs"]
mod git;
#[path = "support/temp_dir.rs"]
mod temp_dir;
use std::{io, path::Path, process::Output};

fn run(repo: &Path, args: &[&str]) -> io::Result<Output> {
    git::isolated_command(env!("CARGO_BIN_EXE_charter"))
        .current_dir(repo)
        .args(args)
        .output()
}
fn init(repo: &Path) -> io::Result<()> {
    std::fs::create_dir(repo.join(".charter"))?;
    for args in [
        &["init"][..],
        &["config", "user.name", "Test"],
        &["config", "core.excludesFile", "/dev/null"],
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
    )
}
fn findings(repo: &Path, names: &[&str]) -> io::Result<()> {
    let results: Vec<_> = names.iter().map(|name| serde_json::json!({"ruleId":"r", "message":{"text":name}, "locations":[{"physicalLocation":{"artifactLocation":{"uri":name}}}]})).collect();
    std::fs::write(
        repo.join("findings.json"),
        serde_json::json!({"runs":[{"results":results}]}).to_string(),
    )
}
const NAMES: [&str; 4] = [
    "staged.rs",
    "space quote'\t\n.rs",
    "-untracked.rs",
    "clean.rs",
];

fn commit_fixture(repo: &Path) -> io::Result<()> {
    for name in [&NAMES[0], &NAMES[1], &NAMES[3]] {
        std::fs::write(repo.join(name), "clean")?;
    }
    findings(repo, &NAMES)?;
    assert!(git::fixture_git(repo, &["add", "."])?.status.success());
    assert!(
        git::fixture_git(repo, &["commit", "-m", "base"])?
            .status
            .success()
    );
    Ok(())
}

#[test]
fn working_tree_includes_staged_unstaged_and_untracked_paths() -> io::Result<()> {
    let temp = temp_dir::TempDir::create()?;
    let repo = temp.0.clone();
    init(&repo)?;
    commit_fixture(&repo)?;
    assert!(run(&repo, &["check", "--working-tree"])?.status.success());
    for name in &NAMES[..3] {
        std::fs::write(repo.join(name), "changed")?;
    }
    assert!(
        git::fixture_git(&repo, &["add", "staged.rs"])?
            .status
            .success()
    );
    verify_selection(&repo)?;
    assert!(
        run(&repo, &["check", "--changed", "HEAD"])?
            .status
            .success()
    );
    let conflict = run(&repo, &["check", "--working-tree", "--changed", "HEAD"])?;
    assert_eq!(conflict.status.code(), Some(2));
    Ok(())
}

fn verify_selection(repo: &Path) -> io::Result<()> {
    let out = run(repo, &["check", "--working-tree", "--json"])?;
    assert_eq!(out.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    let lines = value["lines"]
        .as_array()
        .ok_or_else(|| io::Error::other("missing lines"))?;
    assert_eq!(lines.len(), 3);
    for name in &NAMES[..3] {
        assert!(
            lines
                .iter()
                .any(|line| line.as_str().is_some_and(|text| text.contains(name)))
        );
    }
    Ok(())
}
