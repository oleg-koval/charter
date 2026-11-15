use super::{ImportError, import_failure, terminal_text};
use std::fmt;

const RECIPES: &[(&str, &str)] = &[
    (
        "node-npm",
        include_str!("../../recipes/node-npm/checks.toml"),
    ),
    (
        "clojure-cli-test",
        include_str!("../../recipes/clojure-cli-test/checks.toml"),
    ),
    (
        "go-module",
        include_str!("../../recipes/go-module/checks.toml"),
    ),
];

#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum InitOptionsError {
    ConflictingSources,
    UnknownRecipe(String),
    Import(ImportError),
}

impl fmt::Display for InitOptionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingSources => f.write_str("--recipe conflicts with --checks-from"),
            Self::UnknownRecipe(name) => write!(f, "unknown recipe: {}", terminal_text(name)),
            Self::Import(error) => error.fmt(f),
        }
    }
}

/// # Errors
/// Rejects combining recipe/import sources and names outside the explicit catalog.
pub fn recipe_checks(name: &str, has_import: bool) -> Result<&'static str, InitOptionsError> {
    if has_import {
        return Err(InitOptionsError::ConflictingSources);
    }
    RECIPES
        .iter()
        .find(|(id, _)| *id == name)
        .map(|(_, text)| *text)
        .ok_or_else(|| InitOptionsError::UnknownRecipe(name.to_owned()))
}

#[must_use]
pub fn init_options_failure(error: &InitOptionsError) -> String {
    match error {
        InitOptionsError::Import(error) => import_failure(error),
        _ => serde_json::json!({"event":"charter.init_options.failed", "metric":"charter_init_options_failures", "count":1, "error":error}).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ImportError, parse_mechanism};

    #[test]
    fn catalog_retains_native_commands_and_pending_enforcement()
    -> Result<(), Box<dyn std::error::Error>> {
        for (name, command) in [
            ("node-npm", "npm run build && npm test"),
            ("clojure-cli-test", "clojure -M:test"),
            ("go-module", "go test ./..."),
        ] {
            let text = recipe_checks(name, false).map_err(|error| error.to_string())?;
            let mechanism = parse_mechanism(text)?;
            assert_eq!(
                mechanism.check.first().map(|check| check.run.as_str()),
                Some(command)
            );
            assert!(
                mechanism.check.iter().any(
                    |check| check.id == "configure-enforcement" && check.run.contains("exit 1")
                )
            );
        }
        Ok(())
    }
    #[test]
    fn unknown_names_and_conflicting_sources_are_typed_errors() {
        assert!(matches!(
            recipe_checks("missing", false),
            Err(InitOptionsError::UnknownRecipe(_))
        ));
        assert!(matches!(
            recipe_checks("node-npm", true),
            Err(InitOptionsError::ConflictingSources)
        ));
    }
    #[test]
    fn new_errors_are_observable_and_existing_import_events_stay_compatible()
    -> Result<(), serde_json::Error> {
        let error = InitOptionsError::UnknownRecipe("bad\n\u{1b}".to_owned());
        let event: serde_json::Value = serde_json::from_str(&init_options_failure(&error))?;
        assert_eq!(event["event"], "charter.init_options.failed");
        assert_eq!(event["metric"], "charter_init_options_failures");
        assert_eq!(event["count"], 1);
        assert_eq!(event["error"]["kind"], "unknown_recipe");
        assert!(!error.to_string().contains(['\n', '\u{1b}']));
        let imported = InitOptionsError::Import(ImportError::Read("missing".to_owned()));
        let event: serde_json::Value = serde_json::from_str(&init_options_failure(&imported))?;
        assert_eq!(event["event"], "charter.import.failed");
        assert_eq!(event["error"]["kind"], "read");
        assert_eq!(imported.to_string(), "missing");
        Ok(())
    }
}
