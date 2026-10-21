use std::path::Component;

#[must_use]
pub fn normalise(root: &str, path: &str) -> Option<String> {
    let path = path.strip_prefix("file://").unwrap_or(path);
    let path = path.strip_prefix("./").unwrap_or(path);
    let root = root.trim_end_matches('/');
    let relative = if path.starts_with('/') {
        path.strip_prefix(root)
            .filter(|suffix| suffix.starts_with('/'))?
            .strip_prefix('/')?
    } else {
        path
    };
    if relative.is_empty()
        || std::path::Path::new(relative).components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }
    Some(relative.to_owned())
}

#[must_use]
pub fn same_path(root: &str, left: &str, right: &str) -> bool {
    matches!(
        (normalise(root, left), normalise(root, right)),
        (Some(left), Some(right)) if left == right
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_relative_file_uri_and_absolute_paths() {
        assert_eq!(
            normalise("/repo", "./src/a.rs"),
            Some("src/a.rs".to_owned())
        );
        assert_eq!(
            normalise("/repo", "file:///repo/src/a.rs"),
            Some("src/a.rs".to_owned())
        );
        assert_eq!(normalise("/repo", "/outside/a.rs"), None);
    }
}
