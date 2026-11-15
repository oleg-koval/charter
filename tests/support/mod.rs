use std::io;
use std::path::Path;
use std::process::Command;

pub fn isolated_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    // Hooks export repository selectors that override a fixture's current directory.
    command
        .env_clear()
        .envs(std::env::vars_os().filter(|(key, _)| !key.to_string_lossy().starts_with("GIT_")));
    command
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.hooksPath")
        .env("GIT_CONFIG_VALUE_0", "/dev/null");
    command
}

pub fn fixture_git(repo: &Path, args: &[&str]) -> io::Result<std::process::Output> {
    isolated_command("git")
        .current_dir(repo)
        .args(args)
        .output()
}

fn git(repo: &Path, args: &[&str]) -> io::Result<Vec<u8>> {
    let output = fixture_git(repo, args)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

#[test]
fn fixture_commands_preserve_hook_repository() -> io::Result<()> {
    let outer = std::env::temp_dir().join(format!("charter-hook-{}", std::process::id()));
    std::fs::create_dir_all(&outer)?;
    git(&outer, &["init"])?;
    git(&outer, &["config", "user.name", "Test"])?;
    git(&outer, &["config", "user.email", "test@example.com"])?;
    std::fs::write(outer.join("sentinel"), "original")?;
    git(&outer, &["add", "."])?;
    git(&outer, &["commit", "-m", "sentinel"])?;
    let head = git(&outer, &["rev-parse", "HEAD"])?;
    let index = std::fs::read(outer.join(".git/index"))?;
    let config = std::fs::read(outer.join(".git/config"))?;
    verify_fixtures(&outer, &head, &index, &config)?;
    std::fs::remove_dir_all(outer)
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
            std::ffi::OsStr::new("bash"),
            vec!["scripts/test-preflight-pr-size.sh"],
        ),
        (
            std::ffi::OsStr::new("node"),
            vec!["--test", "github/release.test.mjs"],
        ),
    ];
    for (program, args) in cases {
        run_fixture(outer, program, &args)?;
        assert_preserved(outer, head, index, config)?;
    }
    Ok(())
}

fn run_fixture(outer: &Path, program: &std::ffi::OsStr, args: &[&str]) -> io::Result<()> {
    let output = isolated_command(program)
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("GIT_DIR", outer.join(".git"))
        .env("GIT_WORK_TREE", outer)
        .env("GIT_INDEX_FILE", outer.join(".git/index"))
        .output()?;
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
    Ok(())
}

fn assert_preserved(outer: &Path, head: &[u8], index: &[u8], config: &[u8]) -> io::Result<()> {
    assert_eq!(
        std::fs::read(outer.join(".git/config"))?,
        config,
        "fixture changed hook config"
    );
    assert_eq!(
        git(outer, &["rev-parse", "HEAD"])?,
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
