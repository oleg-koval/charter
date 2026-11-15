use super::{findings, git, init, run, temp_dir::TempDir};
use std::{io, path::Path};

fn repo() -> io::Result<TempDir> {
    let temp = TempDir::create()?;
    init(&temp.0)?;
    Ok(temp)
}
fn commit(repo: &Path) -> io::Result<()> {
    for args in [&["add", "."][..], &["commit", "-m", "fixture"]] {
        assert!(git::fixture_git(repo, args)?.status.success());
    }
    Ok(())
}
fn lines(repo: &Path, expected: &[&str]) -> io::Result<()> {
    let out = run(repo, &["check", "--working-tree", "--json"])?;
    assert_eq!(out.status.success(), expected.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    let wanted: Vec<_> = expected
        .iter()
        .map(|name| format!("scan  {name}:0  r: {name}"))
        .collect();
    assert_eq!(value["lines"], serde_json::json!(wanted));
    Ok(())
}
#[test]
fn unborn_repository_includes_staged_and_untracked_but_not_ignored() -> io::Result<()> {
    let temp = repo()?;
    let repo = temp.0.as_path();
    std::fs::write(repo.join(".gitignore"), "ignored.rs\n")?;
    for name in ["staged.rs", "new.rs", "ignored.rs"] {
        std::fs::write(repo.join(name), "code")?;
    }
    findings(repo, &["staged.rs", "new.rs", "ignored.rs"])?;
    assert!(
        git::fixture_git(repo, &["add", "staged.rs"])?
            .status
            .success()
    );
    lines(repo, &["staged.rs", "new.rs"])?;
    Ok(())
}
#[test]
fn renamed_and_deleted_paths_remain_selected() -> io::Result<()> {
    let temp = repo()?;
    let repo = temp.0.as_path();
    for name in ["old.rs", "deleted.rs", "clean.rs"] {
        std::fs::write(repo.join(name), "code")?;
    }
    findings(repo, &["old.rs", "new.rs", "deleted.rs", "clean.rs"])?;
    commit(repo)?;
    assert!(
        git::fixture_git(repo, &["mv", "old.rs", "new.rs"])?
            .status
            .success()
    );
    std::fs::remove_file(repo.join("deleted.rs"))?;
    lines(repo, &["old.rs", "new.rs", "deleted.rs"])?;
    Ok(())
}
#[test]
fn checks_read_current_content_instead_of_staged_snapshot() -> io::Result<()> {
    let temp = repo()?;
    let repo = temp.0.as_path();
    std::fs::write(
        repo.join(".charter/checks.toml"),
        "[[check]]\nid='content'\nrules=['C1']\nrun='test $(cat source) = compliant'\noutput='exit'",
    )?;
    std::fs::write(repo.join("source"), "compliant")?;
    commit(repo)?;
    std::fs::write(repo.join("source"), "compliant\n")?;
    assert!(git::fixture_git(repo, &["add", "source"])?.status.success());
    assert!(run(repo, &["check", "--working-tree"])?.status.success());
    std::fs::write(repo.join("source"), "violation")?;
    let out = run(repo, &["check", "--working-tree", "--json"])?;
    assert_eq!(out.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(value["lines"], serde_json::json!(["content  exited 1"]));
    assert_eq!(
        git::fixture_git(repo, &["show", ":source"])?.stdout,
        b"compliant\n"
    );
    Ok(())
}
#[test]
fn empty_selection_preserves_failures_and_unresolved_findings() -> io::Result<()> {
    for (label, output, command, content) in [
        ("exit", "exit", "exit 7", "{}"),
        ("malformed", "sarif", "cat findings.json", "invalid json"),
        ("unresolved", "sarif", "cat findings.json", ""),
    ] {
        let temp = repo()?;
        let repo = temp.0.as_path();
        std::fs::write(
            repo.join(".charter/checks.toml"),
            format!("[[check]]\nid='scan'\nrules=['C1']\nrun='{command}'\noutput='{output}'"),
        )?;
        if label == "unresolved" {
            findings(repo, &[""])?;
        } else {
            std::fs::write(repo.join("findings.json"), content)?;
        }
        commit(repo)?;
        assert!(
            git::fixture_git(repo, &["status", "--porcelain"])?
                .stdout
                .is_empty()
        );
        verify_failure(repo, label)?;
    }
    Ok(())
}
fn verify_failure(repo: &Path, label: &str) -> io::Result<()> {
    let out = run(repo, &["check", "--working-tree", "--json"])?;
    assert_eq!(out.status.code(), Some(1));
    if label == "malformed" {
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8_lossy(&out.stderr).contains("scan:"));
    } else {
        let value: serde_json::Value = serde_json::from_slice(&out.stdout)?;
        let line = if label == "exit" {
            "scan  exited 7"
        } else {
            "scan  :0  r: "
        };
        assert_eq!(value["lines"], serde_json::json!([line]));
    }
    Ok(())
}
#[cfg(unix)]
#[test]
fn non_utf8_paths_fail_explicitly() -> io::Result<()> {
    let temp = repo()?;
    let repo = temp.0.as_path();
    findings(repo, &[])?;
    commit(repo)?;
    stage_non_utf8_path(repo)?;
    std::fs::write(
        repo.join(".charter/checks.toml"),
        "[[check]]\nid='scan'\nrules=['C1']\nrun='touch ran'\noutput='exit'",
    )?;
    let out = run(repo, &["check", "--working-tree", "--json"])?;
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("git paths are not valid UTF-8"));
    assert!(!repo.join("ran").exists());
    Ok(())
}

#[cfg(unix)]
fn stage_non_utf8_path(repo: &Path) -> io::Result<()> {
    use std::io::Write;
    use std::process::Stdio;
    let hash = git::fixture_git(repo, &["hash-object", "-w", "--stdin"])?;
    assert!(hash.status.success());
    let record = [b"100644 ".as_slice(), hash.stdout.trim_ascii(), b"\t\xff\0"].concat();
    let mut child = git::isolated_command("git")
        .current_dir(repo)
        .args(["update-index", "-z", "--index-info"])
        .stdin(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("missing stdin"))?
        .write_all(&record)?;
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn discovery_failure_prevents_check_execution() -> io::Result<()> {
    let temp = TempDir::create()?;
    let repo = temp.0.clone();
    init(&repo)?;
    std::fs::remove_dir_all(repo.join(".git"))?;
    std::fs::write(
        repo.join(".charter/checks.toml"),
        "[[check]]\nid='marker'\nrules=['C1']\nrun='touch ran'\noutput='exit'",
    )?;
    let out = run(&repo, &["check", "--working-tree"])?;
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("git rev-parse"));
    assert!(stderr.contains("charter_working_selection_failures"));
    assert!(!repo.join("ran").exists());
    Ok(())
}
