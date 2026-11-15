use super::{commit_fixture, findings, git, init, run, temp_dir::TempDir};
use std::{
    io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

fn fixture() -> io::Result<TempDir> {
    let temp = TempDir::create()?;
    init(&temp.0)?;
    commit_fixture(&temp.0)?;
    Ok(temp)
}
fn text(path: &Path) -> io::Result<&str> {
    path.to_str()
        .ok_or_else(|| io::Error::other("fixture path is not UTF-8"))
}
fn install_monitor(repo: &Path) -> io::Result<PathBuf> {
    let hook = repo.join(".git/monitor");
    let marker = repo.join(".git/monitor-ran");
    std::fs::write(&hook, "#!/bin/sh\ntouch .git/monitor-ran\nprintf '\\0'\n")?;
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
    assert!(
        git::fixture_git(repo, &["config", "core.fsmonitor", text(&hook)?])?
            .status
            .success()
    );
    Ok(marker)
}
#[test]
fn configured_fsmonitor_is_not_executed() -> io::Result<()> {
    let temp = fixture()?;
    let repo = temp.0.as_path();
    let marker = install_monitor(repo)?;
    assert!(
        git::fixture_git(repo, &["status", "--porcelain=v1", "-z"])?
            .status
            .success()
    );
    assert!(marker.exists(), "ordinary Git must execute the probe");
    std::fs::remove_file(&marker)?;
    assert!(
        run(repo, &["check", "--working-tree", "--json"])?
            .status
            .success()
    );
    assert!(
        !marker.exists(),
        "Charter must disable configured fsmonitor"
    );
    Ok(())
}
#[test]
fn alternate_index_agrees_with_git_from_a_subdirectory() -> io::Result<()> {
    let temp = fixture()?;
    let repo = temp.0.as_path();
    let nested = repo.join("nested");
    std::fs::create_dir(&nested)?;
    let index = repo.join(".git/alternate-index");
    std::fs::copy(repo.join(".git/index"), &index)?;
    std::fs::write(repo.join("staged.rs"), "different")?;
    assert!(
        git::fixture_git(repo, &["add", "staged.rs"])?
            .status
            .success()
    );
    std::fs::write(repo.join("staged.rs"), "clean")?;
    std::fs::write(
        repo.join(".charter/checks.toml"),
        "[[check]]\nid='scan'\nrules=['C1']\nrun='cat ../findings.json'\noutput='sarif'",
    )?;
    verify_owned_index(&nested, &index)?;
    for (selected_index, expected) in [(".git/index", 1), (".git/alternate-index", 0)] {
        verify_index(&nested, selected_index, expected)?;
    }
    Ok(())
}
fn verify_owned_index(nested: &Path, index: &Path) -> io::Result<()> {
    let path = git::isolated_command("git")
        .current_dir(nested)
        .env("GIT_INDEX_FILE", ".git/alternate-index")
        .args(["rev-parse", "--git-path", "index"])
        .output()?;
    assert!(path.status.success());
    let resolved = nested
        .join(String::from_utf8_lossy(&path.stdout).trim())
        .canonicalize()?;
    assert_eq!(resolved, index.canonicalize()?);
    Ok(())
}
fn selected_by_git(nested: &Path, index: &str) -> io::Result<bool> {
    let direct = git::isolated_command("git")
        .current_dir(nested)
        .env("GIT_INDEX_FILE", index)
        .args([
            "-c",
            "core.fsmonitor=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--no-renames",
            "--ignore-submodules=none",
        ])
        .output()?;
    assert!(direct.status.success());
    let selected = direct
        .stdout
        .split(|byte| *byte == 0)
        .any(|record| record.get(3..) == Some(b"staged.rs"));
    Ok(selected)
}
fn verify_index(nested: &Path, index: &str, expected: usize) -> io::Result<()> {
    let selected = selected_by_git(nested, index)?;
    assert_eq!(selected, expected == 1);
    let out = git::isolated_command(env!("CARGO_BIN_EXE_charter"))
        .current_dir(nested)
        .env("GIT_INDEX_FILE", index)
        .args(["check", "--dir", "../.charter", "--working-tree", "--json"])
        .output()?;
    assert_eq!(out.status.success(), expected == 0);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(value["lines"].as_array().map(Vec::len), Some(expected));
    Ok(())
}
fn add_submodule(repo: &Path, source: &Path) -> io::Result<()> {
    assert!(
        git::fixture_git(
            repo,
            &[
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "add",
                "--",
                text(source)?,
                "module"
            ]
        )?
        .status
        .success()
    );
    Ok(())
}
fn commit_ignored_submodule(repo: &Path) -> io::Result<()> {
    findings(repo, &["module"])?;
    for args in [
        &["add", "."][..],
        &["commit", "-m", "submodule"],
        &["config", "diff.ignoreSubmodules", "all"],
    ] {
        assert!(git::fixture_git(repo, args)?.status.success());
    }
    Ok(())
}
#[test]
fn dirty_submodule_is_selected_despite_repository_ignore_setting() -> io::Result<()> {
    let source = fixture()?;
    let temp = fixture()?;
    let repo = temp.0.as_path();
    add_submodule(repo, &source.0)?;
    commit_ignored_submodule(repo)?;
    std::fs::write(repo.join("module/staged.rs"), "modified")?;
    let hidden = git::fixture_git(repo, &["status", "--porcelain=v1", "-z"])?;
    assert!(hidden.status.success());
    assert!(
        hidden.stdout.is_empty(),
        "fixture must demonstrate the configured hiding behavior"
    );
    let out = run(repo, &["check", "--working-tree", "--json"])?;
    assert_eq!(out.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(
        value["lines"],
        serde_json::json!(["scan  module:0  r: module"])
    );
    Ok(())
}
