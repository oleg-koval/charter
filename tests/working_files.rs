#[path = "support/git.rs"]
mod git;
#[path = "support/temp_dir.rs"]
mod temp_dir;
use charter_cli::{domain::GitPathsError, edge::working_files};
use std::{fs, io, path::Path};

fn git_ok(root: &Path, args: &[&str]) -> io::Result<()> {
    let output = git::fixture_git(root, args)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn repository() -> io::Result<temp_dir::TempDir> {
    let temp = temp_dir::TempDir::create()?;
    let root = temp.0.as_path();
    for args in [
        &["init"][..],
        &["config", "user.name", "Test"],
        &["config", "user.email", "test@example.com"],
    ] {
        git_ok(root, args)?;
    }
    for name in ["staged.rs", "staged-only.rs", "unstaged.rs", "deleted.rs"] {
        fs::write(root.join(name), "old")?;
    }
    fs::write(root.join(".gitignore"), "ignored.rs\n")?;
    git_ok(root, &["add", "."])?;
    git_ok(root, &["commit", "-m", "initial"])?;
    Ok(temp)
}

fn root_text(root: &Path) -> io::Result<&str> {
    root.to_str()
        .ok_or_else(|| io::Error::other("non-UTF8 fixture root"))
}

#[test]
#[ignore = "executed in a Git-isolated child"]
fn api_child_working_paths() -> io::Result<()> {
    let temp = repository()?;
    let root = temp.0.as_path();
    let cwd = std::env::current_dir()?;
    assert!(
        working_files(root_text(root)?)
            .map_err(|error| io::Error::other(error.to_string()))?
            .is_empty()
    );
    change_files(root)?;
    assert_eq!(
        working_files(root_text(root)?).map_err(|error| io::Error::other(error.to_string()))?,
        [
            "deleted.rs",
            "staged-only.rs",
            "staged.rs",
            "unstaged.rs",
            "untracked.rs"
        ]
    );
    assert_eq!(std::env::current_dir()?, cwd);
    Ok(())
}

#[test]
#[ignore = "executed in a Git-isolated child"]
fn api_child_missing_root() -> io::Result<()> {
    let temp = temp_dir::TempDir::create()?;
    assert!(matches!(
        working_files(root_text(&temp.0.join("missing"))?),
        Err(GitPathsError::Spawn(_))
    ));
    Ok(())
}

fn change_files(root: &Path) -> io::Result<()> {
    fs::write(root.join("staged.rs"), "staged")?;
    fs::write(root.join("staged-only.rs"), "staged")?;
    git_ok(root, &["add", "staged.rs", "staged-only.rs"])?;
    fs::write(root.join("staged.rs"), "also unstaged")?;
    fs::write(root.join("unstaged.rs"), "changed")?;
    fs::remove_file(root.join("deleted.rs"))?;
    fs::write(root.join("untracked.rs"), "new")?;
    fs::write(root.join("ignored.rs"), "ignored")?;
    Ok(())
}

#[test]
fn isolated_api_proof() -> io::Result<()> {
    let output = git::isolated_command(std::env::current_exe()?)
        .args(["--ignored", "api_child_"])
        .output()?;
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test api_child_working_paths ... ok"));
    assert!(stdout.contains("test api_child_missing_root ... ok"));
    assert!(stdout.contains("2 passed; 0 failed"));
    Ok(())
}

#[test]
fn exported_hook_selectors_do_not_redirect_api_proof() -> io::Result<()> {
    let outer = repository()?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", "isolated_api_proof"])
        .env("GIT_DIR", outer.0.join(".git"))
        .env("GIT_WORK_TREE", &outer.0)
        .env("GIT_INDEX_FILE", outer.0.join(".git/index"))
        .output()?;
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("test isolated_api_proof ... ok"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
    Ok(())
}
