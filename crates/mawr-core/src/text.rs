use std::fmt;

use crate::{ValidationError, ValidationIssue};

/// Text with an explicit UTF-8 byte bound. Empty text is allowed.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundedText<const MAX_BYTES: usize>(String);

impl<const MAX_BYTES: usize> BoundedText<MAX_BYTES> {
    pub fn new(value: impl Into<String>, field: &'static str) -> Result<Self, ValidationError> {
        let value = value.into();
        validate_length(&value, MAX_BYTES, field)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<const MAX_BYTES: usize> fmt::Debug for BoundedText<MAX_BYTES> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<const MAX_BYTES: usize> fmt::Display for BoundedText<MAX_BYTES> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Non-whitespace text with an explicit UTF-8 byte bound.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonEmptyText<const MAX_BYTES: usize>(String);

impl<const MAX_BYTES: usize> NonEmptyText<MAX_BYTES> {
    pub fn new(value: impl Into<String>, field: &'static str) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ValidationError::new(field, ValidationIssue::Empty));
        }
        validate_length(&value, MAX_BYTES, field)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<const MAX_BYTES: usize> fmt::Debug for NonEmptyText<MAX_BYTES> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<const MAX_BYTES: usize> fmt::Display for NonEmptyText<MAX_BYTES> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Potentially secret text that never reveals its contents through `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct SensitiveText<const MAX_BYTES: usize>(String);

impl<const MAX_BYTES: usize> SensitiveText<MAX_BYTES> {
    pub fn new(value: impl Into<String>, field: &'static str) -> Result<Self, ValidationError> {
        let value = value.into();
        validate_length(&value, MAX_BYTES, field)?;
        Ok(Self(value))
    }

    /// Explicitly exposes the secret at the execution boundary.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl<const MAX_BYTES: usize> fmt::Debug for SensitiveText<MAX_BYTES> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SensitiveText(<redacted>)")
    }
}

fn validate_length(
    value: &str,
    max_bytes: usize,
    field: &'static str,
) -> Result<(), ValidationError> {
    if value.len() > max_bytes {
        return Err(ValidationError::new(
            field,
            ValidationIssue::TooLong {
                max_bytes,
                actual_bytes: value.len(),
            },
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{BoundedText, NonEmptyText, SensitiveText};

    #[test]
    fn validates_utf8_by_bytes() {
        assert!(BoundedText::<4>::new("éé", "text").is_ok());
        assert!(BoundedText::<3>::new("éé", "text").is_err());
    }

    #[test]
    fn non_empty_rejects_whitespace_only() {
        assert!(NonEmptyText::<8>::new(" \n ", "name").is_err());
    }

    #[test]
    fn sensitive_debug_is_redacted() {
        let value = SensitiveText::<32>::new("do-not-log", "value").unwrap();
        assert_eq!(format!("{value:?}"), "SensitiveText(<redacted>)");
        assert!(!format!("{value:?}").contains(value.expose_secret()));
    }
}
