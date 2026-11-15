#[path = "support/mod.rs"]
mod git;
#[path = "support/temp_dir.rs"]
mod temp_dir;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Output;

const ORIGINAL: &str = r#"{"version":1,"checks":{"chosen":["old"],"other":["keep"]}}"#;
const SARIF: &str = r#"{"runs":[{"results":[]}]}"#;

fn check(id: &str, output: &str) -> String {
    format!(
        "[[check]]\nid = '{id}'\nrules = []\nrun = 'touch {id}; cat findings.json'\noutput = '{output}'\n"
    )
}

fn run(root: &Path, args: &[&str]) -> io::Result<Output> {
    git::isolated_command(env!("CARGO_BIN_EXE_charter"))
        .current_dir(root)
        .args(["baseline", "--dir", "."])
        .args(args)
        .output()
}

fn fixture() -> io::Result<temp_dir::TempDir> {
    let temp = temp_dir::TempDir::create()?;
    fs::write(temp.0.join("baseline.json"), ORIGINAL)?;
    fs::write(temp.0.join("findings.json"), SARIF)?;
    Ok(temp)
}

#[test]
fn preview_and_selected_update_preserve_unrelated_baselines() -> io::Result<()> {
    let temp = fixture()?;
    let root = temp.0.as_path();
    fs::write(
        root.join("checks.toml"),
        check("chosen", "sarif") + &check("other", "sarif"),
    )?;
    let proposed = preview(root)?;
    assert_eq!(
        proposed,
        serde_json::json!({"version":1,"checks":{"chosen":[],"other":["keep"]}})
    );
    assert!(run(root, &["--check", "chosen"])?.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(root.join("baseline.json"))?)?,
        proposed
    );
    Ok(())
}

fn preview(root: &Path) -> io::Result<serde_json::Value> {
    let preview = run(root, &["--check", "chosen", "--preview"])?;
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    assert_eq!(fs::read_to_string(root.join("baseline.json"))?, ORIGINAL);
    assert!(root.join("chosen").exists());
    assert!(!root.join("other").exists());
    Ok(serde_json::from_slice(&preview.stdout)?)
}

#[test]
fn preview_does_not_create_a_missing_baseline() -> io::Result<()> {
    let temp = fixture()?;
    let root = temp.0.as_path();
    fs::write(root.join("checks.toml"), check("chosen", "sarif"))?;
    fs::remove_file(root.join("baseline.json"))?;
    assert!(
        run(root, &["--check", "chosen", "--preview"])?
            .status
            .success()
    );
    assert!(!root.join("baseline.json").exists());
    Ok(())
}

#[test]
fn invalid_selection_and_baseline_never_execute_tools() -> io::Result<()> {
    let temp = fixture()?;
    let root = temp.0.as_path();
    for config in [
        check("other", "sarif"),
        check("chosen", "exit"),
        check("chosen", "sarif") + &check("chosen", "sarif"),
    ] {
        fs::write(root.join("checks.toml"), config)?;
        let result = run(root, &["--check", "chosen"])?;
        assert_eq!(result.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("charter_baseline_selection_failures")
        );
        assert!(!root.join("chosen").exists());
        assert!(!root.join("other").exists());
        assert_eq!(fs::read_to_string(root.join("baseline.json"))?, ORIGINAL);
    }
    fs::write(root.join("checks.toml"), check("chosen", "sarif"))?;
    fs::write(root.join("baseline.json"), "invalid")?;
    let result = run(root, &[])?;
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("invalid baseline"));
    assert!(!root.join("chosen").exists());
    assert_eq!(fs::read_to_string(root.join("baseline.json"))?, "invalid");
    Ok(())
}
