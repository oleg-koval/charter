#![cfg(unix)]
#[path = "support/mod.rs"]
mod command;
#[path = "support/temp_dir.rs"]
mod temp_dir;
use std::{
    io,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
    process::Output,
};

fn charter(dir: &Path, args: &[&str]) -> io::Result<Output> {
    command::isolated_command(env!("CARGO_BIN_EXE_charter"))
        .current_dir(dir)
        .env("PATH", dir.join("bin"))
        .args(args)
        .output()
}
fn import(dir: &Path) -> io::Result<()> {
    let source = format!(
        "{}/recipes/clojure-cli-test/checks.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    let out = charter(dir, &["init", "--checks-from", &source])?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        std::fs::read(&source)?,
        std::fs::read(dir.join(".charter/checks.toml"))?
    );
    assert!(charter(dir, &["doctor"])?.status.success());
    let text = std::fs::read_to_string(source)?;
    let mechanism = charter_cli::domain::parse_mechanism(&text).map_err(io::Error::other)?;
    assert!(mechanism.waive.is_empty());
    assert_eq!(mechanism.check.len(), 2);
    assert_eq!(mechanism.check[0].rules, ["C7"]);
    assert_eq!(
        mechanism.check[1].rules,
        ["C1", "C2", "C3", "C4", "C5", "C10", "C23"]
    );
    Ok(())
}
fn shim(dir: &Path, code: u8) -> io::Result<()> {
    let path = dir.join("bin/clojure");
    std::fs::write(
        &path,
        format!("#!/bin/sh\nprintf '%s\\n' \"$*\" \"$PWD\" > invoked\nexit {code}\n"),
    )?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
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
fn clojure_recipe_wiring_preserves_failures_and_missing_tool() -> io::Result<()> {
    let temp = temp_dir::TempDir::create()?;
    let dir = &temp.0;
    std::fs::create_dir_all(dir.join("bin"))?;
    symlink("/bin/sh", dir.join("bin/sh"))?;
    import(dir)?;
    shim(dir, 0)?;
    check(dir, &["configure-enforcement  exited 1"])?;
    assert_eq!(
        std::fs::read_to_string(dir.join("invoked"))?,
        format!("-M:test\n{}\n", std::fs::canonicalize(dir)?.display())
    );
    shim(dir, 9)?;
    check(
        dir,
        &["clojure-tests  exited 9", "configure-enforcement  exited 1"],
    )?;
    std::fs::remove_file(dir.join("bin/clojure"))?;
    check(
        dir,
        &[
            "clojure-tests  exited 127",
            "configure-enforcement  exited 1",
        ],
    )?;
    Ok(())
}
