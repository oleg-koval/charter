#[path = "support/mod.rs"]
mod command;
#[path = "support/temp_dir.rs"]
mod temp_dir;

use std::{io, path::Path, process::Output};

fn charter(dir: &Path, args: &[&str]) -> io::Result<Output> {
    command::isolated_command(env!("CARGO_BIN_EXE_charter"))
        .current_dir(dir)
        .envs([
            ("GOWORK", "off"),
            ("GOENV", "off"),
            ("GOTOOLCHAIN", "local"),
            ("GOPROXY", "off"),
            ("GOSUMDB", "off"),
        ])
        .args(args)
        .output()
}
fn import(dir: &Path) -> io::Result<()> {
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/recipes/go-module/checks.toml");
    let out = charter(dir, &["init", "--checks-from", source])?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        std::fs::read(source)?,
        std::fs::read(dir.join(".charter/checks.toml"))?
    );
    assert!(charter(dir, &["doctor"])?.status.success());
    let text = std::fs::read_to_string(source)?;
    let mechanism = charter_cli::domain::parse_mechanism(&text).map_err(io::Error::other)?;
    assert!(mechanism.waive.is_empty());
    assert_eq!(mechanism.check.len(), 2);
    assert_eq!(mechanism.check[0].rules, ["C7"]);
    assert_eq!(mechanism.check[0].run, "go test ./...");
    assert_eq!(
        mechanism.check[1].rules,
        ["C1", "C2", "C3", "C4", "C5", "C10", "C23"]
    );
    Ok(())
}
fn check(dir: &Path, expected: &[&str]) -> io::Result<()> {
    let out = charter(dir, &["check", "--json"])?;
    assert_eq!(out.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(json["lines"], serde_json::json!(expected));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("Configure project source and secret checks")
    );
    Ok(())
}
#[test]
fn go_recipe_imports_actual_file_with_honest_mappings() -> io::Result<()> {
    let temp = temp_dir::TempDir::create()?;
    let dir = &temp.0;
    import(dir)?;
    Ok(())
}

#[test]
#[ignore = "requires Go; run cargo test --test go_recipe -- --ignored"]
fn go_recipe_runs_native_tests_and_retains_pending_enforcement() -> io::Result<()> {
    let temp = temp_dir::TempDir::create()?;
    let dir = &temp.0;
    import(dir)?;
    std::fs::write(
        dir.join("go.mod"),
        "module example.invalid/recipe\n\ngo 1.18\n",
    )?;
    std::fs::write(dir.join("value.go"), "package recipe\nconst Value = 2\n")?;
    std::fs::write(
        dir.join("value_test.go"),
        "package recipe\nimport \"testing\"\nfunc TestValue(t *testing.T) { if Value != 2 { t.Fatal(\"wrong value\") } }\n",
    )?;
    check(dir, &["configure-enforcement  exited 1"])?;
    std::fs::write(dir.join("value.go"), "package recipe\nconst Value = 1\n")?;
    check(
        dir,
        &["go-tests  exited 1", "configure-enforcement  exited 1"],
    )?;
    Ok(())
}
