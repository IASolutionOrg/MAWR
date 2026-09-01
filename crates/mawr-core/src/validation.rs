use std::fmt;

/// A secret-safe description of why a domain value was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    field: &'static str,
    issue: ValidationIssue,
}

impl ValidationError {
    pub(crate) const fn new(field: &'static str, issue: ValidationIssue) -> Self {
        Self { field, issue }
    }

    /// Returns the stable field or invariant name without exposing its value.
    #[must_use]
    pub const fn field(&self) -> &'static str {
        self.field
    }

    #[must_use]
    pub const fn issue(&self) -> &ValidationIssue {
        &self.issue
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid {}: {}", self.field, self.issue)
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationIssue {
    Empty,
    TooLong {
        max_bytes: usize,
        actual_bytes: usize,
    },
    Zero,
    OutOfRange {
        min: u64,
        max: u64,
        actual: u64,
    },
    InvalidBounds {
        min: u64,
        max: u64,
    },
    InvalidFormat,
    ForbiddenCharacter {
        byte_index: usize,
    },
    SessionMismatch {
        expected: u64,
        actual: u64,
    },
    Duplicate,
    InvalidTransition,
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("must not be empty"),
            Self::TooLong {
                max_bytes,
                actual_bytes,
            } => write!(
                formatter,
                "is {actual_bytes} bytes, exceeding the {max_bytes}-byte limit"
            ),
            Self::Zero => formatter.write_str("must be non-zero"),
            Self::OutOfRange { min, max, actual } => {
                write!(formatter, "{actual} is outside {min}..={max}")
            }
            Self::InvalidBounds { min, max } => {
                write!(formatter, "declared minimum {min} exceeds maximum {max}")
            }
            Self::InvalidFormat => formatter.write_str("has an invalid format"),
            Self::ForbiddenCharacter { byte_index } => {
                write!(
                    formatter,
                    "contains a forbidden character at byte {byte_index}"
                )
            }
            Self::SessionMismatch { expected, actual } => {
                write!(
                    formatter,
                    "belongs to session {actual}, expected {expected}"
                )
            }
            Self::Duplicate => formatter.write_str("duplicates an existing value"),
            Self::InvalidTransition => formatter.write_str("does not describe a valid transition"),
        }
    }
}
