use super::support::isolated_command;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn git(repo: &Path, args: &[&str]) -> io::Result<Output> {
    isolated_command("git")
        .current_dir(repo)
        .args(args)
        .output()
}

fn git_stdout(repo: &Path, args: &[&str]) -> io::Result<Vec<u8>> {
    let output = git(repo, args)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

struct TempRepo(PathBuf);

impl TempRepo {
    fn create() -> io::Result<Self> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let name = format!("charter-hook-{}-{nanos}", std::process::id());
        let path = std::env::temp_dir().join(name);
        // create_dir, not create_dir_all: a leftover path is an error, never reused.
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            eprintln!("could not remove {}: {error}", self.0.display());
        }
    }
}

#[test]
fn fixture_commands_preserve_hook_repository() -> io::Result<()> {
    let temp = TempRepo::create()?;
    let outer = temp.0.as_path();
    git_stdout(outer, &["init"])?;
    git_stdout(outer, &["config", "user.name", "Test"])?;
    git_stdout(outer, &["config", "user.email", "test@example.com"])?;
    std::fs::write(outer.join("sentinel"), "original")?;
    git_stdout(outer, &["add", "."])?;
    git_stdout(outer, &["commit", "-m", "sentinel"])?;
    let head = git_stdout(outer, &["rev-parse", "HEAD"])?;
    let index = std::fs::read(outer.join(".git/index"))?;
    let config = std::fs::read(outer.join(".git/config"))?;
    verify_fixtures(outer, &head, &index, &config)
}

fn verify_fixtures(outer: &Path, head: &[u8], index: &[u8], config: &[u8]) -> io::Result<()> {
    let test_binary = std::env::current_exe()?;
    let cases = [
        (
            test_binary.as_os_str(),
            vec![
                "--exact",
                "check_changed_filters_sarif_to_committed_changed_paths",
            ],
        ),
        (
            OsStr::new("bash"),
            vec!["scripts/test-preflight-pr-size.sh"],
        ),
        (
            OsStr::new("node"),
            vec!["--test", "github/release.test.mjs"],
        ),
    ];
    for (program, args) in cases {
        if run_fixture(outer, program, &args)? {
            assert_preserved(outer, head, index, config)?;
        }
    }
    Ok(())
}

fn spawn_fixture(outer: &Path, program: &OsStr, args: &[&str]) -> io::Result<Option<Output>> {
    let result = isolated_command(program)
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("GIT_DIR", outer.join(".git"))
        .env("GIT_WORK_TREE", outer)
        .env("GIT_INDEX_FILE", outer.join(".git/index"))
        .output();
    match result {
        Ok(output) => Ok(Some(output)),
        // bash and node are optional for `cargo test`; their own suites cover them.
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            eprintln!("skipping {} fixture: {error}", program.to_string_lossy());
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn run_fixture(outer: &Path, program: &OsStr, args: &[&str]) -> io::Result<bool> {
    let Some(output) = spawn_fixture(outer, program, args)? else {
        return Ok(false);
    };
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if args.first() == Some(&"--exact") {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("test check_changed_filters_sarif_to_committed_changed_paths ... ok")
        );
        assert!(stdout.contains("1 passed; 0 failed"));
    }
    Ok(true)
}

fn assert_preserved(outer: &Path, head: &[u8], index: &[u8], config: &[u8]) -> io::Result<()> {
    assert_eq!(
        std::fs::read_to_string(outer.join("sentinel"))?,
        "original",
        "fixture changed hook worktree"
    );
    assert_eq!(
        std::fs::read(outer.join(".git/config"))?,
        config,
        "fixture changed hook config"
    );
    assert_eq!(
        git_stdout(outer, &["rev-parse", "HEAD"])?,
        head,
        "fixture moved hook HEAD"
    );
    assert_eq!(
        std::fs::read(outer.join(".git/index"))?,
        index,
        "fixture changed hook index"
    );
    Ok(())
}
