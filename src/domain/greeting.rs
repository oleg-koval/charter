use std::fmt;

const MAX_NAME_CHARS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GreetError {
    EmptyName,
    NameTooLong { chars: usize, max: usize },
}

impl fmt::Display for GreetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "name must not be empty"),
            Self::NameTooLong { chars, max } => {
                write!(f, "name has {chars} characters, max is {max}")
            }
        }
    }
}

impl std::error::Error for GreetError {}

/// Builds the greeting for `name`.
///
/// # Errors
///
/// Returns [`GreetError`] when the trimmed name is empty or longer than 64 characters.
pub fn greeting(name: &str) -> Result<String, GreetError> {
    let trimmed = name.trim();
    let chars = trimmed.chars().count();
    match chars {
        0 => Err(GreetError::EmptyName),
        n if n > MAX_NAME_CHARS => Err(GreetError::NameTooLong {
            chars: n,
            max: MAX_NAME_CHARS,
        }),
        _ => Ok(format!("Hello, {trimmed}!")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greets_a_trimmed_name() {
        assert_eq!(greeting("  Ada "), Ok("Hello, Ada!".to_owned()));
    }

    #[test]
    fn rejects_an_empty_name() {
        assert_eq!(greeting("   "), Err(GreetError::EmptyName));
    }

    #[test]
    fn counts_characters_not_bytes() {
        let name = "é".repeat(MAX_NAME_CHARS);
        assert!(greeting(&name).is_ok());
        assert_eq!(
            greeting(&format!("{name}é")),
            Err(GreetError::NameTooLong {
                chars: MAX_NAME_CHARS + 1,
                max: MAX_NAME_CHARS
            })
        );
    }
}
