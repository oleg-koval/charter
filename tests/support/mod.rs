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
