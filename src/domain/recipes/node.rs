use super::{Check, ImportError, InitOptionsError, Mechanism};
use std::collections::BTreeSet;

pub(super) fn native_command(package_json: &str) -> Result<String, InitOptionsError> {
    let manifest = serde_json::from_str::<serde_json::Value>(package_json).map_err(|error| {
        InitOptionsError::Import(ImportError::Parse(format!("package.json: {error}")))
    })?;
    let scripts = manifest
        .get("scripts")
        .and_then(serde_json::Value::as_object);
    let tests = test_commands(scripts)?;
    let build = scripts
        .and_then(|scripts| scripts.get("build"))
        .filter(|command| command.is_string())
        .map(|_| "npm run build".to_owned());
    Ok(build
        .into_iter()
        .chain(tests)
        .collect::<Vec<_>>()
        .join(" && "))
}

pub(super) fn with_command(mechanism: &Mechanism, command: &str) -> Mechanism {
    Mechanism {
        check: mechanism
            .check
            .iter()
            .map(|check| {
                if check.id == "node-build-test" {
                    Check {
                        run: command.to_owned(),
                        ..check.clone()
                    }
                } else {
                    check.clone()
                }
            })
            .collect(),
        ..mechanism.clone()
    }
}

fn test_commands(
    scripts: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Result<Vec<String>, InitOptionsError> {
    let names = scripts
        .into_iter()
        .flat_map(|scripts| scripts.iter())
        .filter(|(name, command)| {
            (name.as_str() == "test" || name.starts_with("test:")) && command.is_string()
        })
        .map(|(name, _)| name.as_str())
        .collect::<BTreeSet<_>>();
    if names.is_empty() {
        return Err(recipe_parse_error(
            "package.json: declare a test or test:* script for automatic Node detection",
        ));
    }
    Ok(if names.contains("test") {
        vec!["npm test".to_owned()]
    } else {
        names
            .iter()
            .map(|name| format!("npm run '{}'", shell_quote(name)))
            .collect()
    })
}

fn shell_quote(word: &str) -> String {
    word.replace('\'', "'\\''")
}

fn recipe_parse_error(message: &str) -> InitOptionsError {
    InitOptionsError::Import(ImportError::Parse(message.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_detection_uses_declared_tests_and_quotes_names() -> Result<(), String> {
        assert_eq!(
            native_command(
                r#"{"scripts":{"build":"make","test":"make test","test:unit":"make unit"}}"#
            )
            .map_err(|error| error.to_string())?,
            "npm run build && npm test"
        );
        assert_eq!(
            native_command(r#"{"scripts":{"test:unit":"make unit","test:assets":"make assets"}}"#)
                .map_err(|error| error.to_string())?,
            "npm run 'test:assets' && npm run 'test:unit'"
        );
        assert_eq!(
            native_command(r#"{"scripts":{"test:unit'$(touch /tmp/not-run)":"echo unit"}}"#)
                .map_err(|error| error.to_string())?,
            "npm run 'test:unit'\\''$(touch /tmp/not-run)'"
        );
        assert!(native_command(r#"{"scripts":{"build":"make"}}"#).is_err());
        Ok(())
    }
}
