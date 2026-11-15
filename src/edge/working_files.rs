use crate::domain::GitPathsError;

/// Returns staged, unstaged and non-ignored untracked paths relative to the repository root.
///
/// # Errors
/// Returns the first Git spawn, exit or UTF-8 decoding error.
pub fn working_files(root: &str) -> Result<Vec<String>, GitPathsError> {
    let commands: &[&[&str]] = &[
        &["diff", "--name-only", "-z", "--no-renames", "--"],
        &[
            "diff",
            "--cached",
            "--name-only",
            "-z",
            "--no-renames",
            "--",
        ],
        &["ls-files", "--others", "--exclude-standard", "-z", "--"],
    ];
    commands
        .iter()
        .map(|args| super::git_paths::paths(root, args))
        .collect::<Result<Vec<_>, _>>()
        .map(|groups| {
            groups
                .into_iter()
                .flatten()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect()
        })
}
