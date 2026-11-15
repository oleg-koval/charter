#[path = "mod.rs"]
mod command;

pub use command::isolated_command;
use std::io;
use std::path::Path;
use std::process::Output;

pub fn fixture_git(repo: &Path, args: &[&str]) -> io::Result<Output> {
    isolated_command("git")
        .current_dir(repo)
        .args(args)
        .output()
}
