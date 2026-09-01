use std::fmt;

use crate::{NonEmptyText, ValidationError, ValidationIssue};

pub const MAX_URL_BYTES: usize = 8_192;

/// An absolute URL-shaped value owned by MAWR.
///
/// This type enforces transport-independent safety invariants, not complete URL
/// parsing or authorization. Engine boundaries remain responsible for parsing,
/// resolution, canonicalization, destination policy, and DNS checks.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AbsoluteUrl(NonEmptyText<MAX_URL_BYTES>);

impl AbsoluteUrl {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        let colon = value
            .find(':')
            .ok_or_else(|| ValidationError::new("url", ValidationIssue::InvalidFormat))?;

        let scheme = &value[..colon];
        let remainder = &value[colon + 1..];
        if scheme.is_empty()
            || remainder.is_empty()
            || !scheme.as_bytes()[0].is_ascii_alphabetic()
            || !scheme
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
        {
            return Err(ValidationError::new("url", ValidationIssue::InvalidFormat));
        }

        if let Some((index, _)) = value.char_indices().find(|(_, character)| {
            character.is_ascii_control() || character.is_ascii_whitespace() || *character == '\\'
        }) {
            return Err(ValidationError::new(
                "url",
                ValidationIssue::ForbiddenCharacter { byte_index: index },
            ));
        }

        let normalized = format!("{}{}", scheme.to_ascii_lowercase(), &value[colon..]);
        Ok(Self(NonEmptyText::new(normalized, "url")?))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[must_use]
    pub fn scheme(&self) -> &str {
        self.as_str()
            .split_once(':')
            .map_or("", |(scheme, _)| scheme)
    }
}

impl fmt::Debug for AbsoluteUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("AbsoluteUrl")
            .field(&format_args!("{}:<redacted>", self.scheme()))
            .finish()
    }
}

impl fmt::Display for AbsoluteUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::AbsoluteUrl;

    #[test]
    fn requires_absolute_shape_and_rejects_ambiguous_characters() {
        assert!(AbsoluteUrl::new("relative/path").is_err());
        assert!(AbsoluteUrl::new("https://example.test/a b").is_err());
        assert!(AbsoluteUrl::new("https:\\example.test").is_err());
        assert!(AbsoluteUrl::new("https://example.test").is_ok());
    }

    #[test]
    fn normalizes_only_the_scheme() {
        let url = AbsoluteUrl::new("HTTPS://Example.test/Path").unwrap();
        assert_eq!(url.as_str(), "https://Example.test/Path");
        assert_eq!(url.scheme(), "https");
    }

    #[test]
    fn debug_does_not_expose_credentials_or_query_values() {
        let url = AbsoluteUrl::new("https://user:secret@example.test/?token=hidden").unwrap();
        let debug = format!("{url:?}");

        assert_eq!(debug, "AbsoluteUrl(https:<redacted>)");
        assert!(!debug.contains("secret"));
        assert!(!debug.contains("hidden"));
    }
}
