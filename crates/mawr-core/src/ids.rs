use std::fmt;
use std::num::{NonZeroU32, NonZeroU64};

use crate::{ValidationError, ValidationIssue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(NonZeroU64);

impl SessionId {
    pub fn new(value: u64) -> Result<Self, ValidationError> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or_else(|| ValidationError::new("session_id", ValidationIssue::Zero))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

macro_rules! scoped_u64_id {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name {
            session: SessionId,
            sequence: NonZeroU64,
        }

        impl $name {
            pub fn new(session: SessionId, sequence: u64) -> Result<Self, ValidationError> {
                let sequence = NonZeroU64::new(sequence)
                    .ok_or_else(|| ValidationError::new($field, ValidationIssue::Zero))?;
                Ok(Self { session, sequence })
            }

            #[must_use]
            pub const fn session(self) -> SessionId {
                self.session
            }

            #[must_use]
            pub const fn sequence(self) -> u64 {
                self.sequence.get()
            }
        }
    };
}

scoped_u64_id!(StateId, "state_id");
scoped_u64_id!(PageId, "page_id");

/// A compact reference whose equality includes its owning session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementRef {
    session: SessionId,
    sequence: NonZeroU32,
}

impl ElementRef {
    pub fn new(session: SessionId, sequence: u32) -> Result<Self, ValidationError> {
        let sequence = NonZeroU32::new(sequence)
            .ok_or_else(|| ValidationError::new("element_ref", ValidationIssue::Zero))?;
        Ok(Self { session, sequence })
    }

    #[must_use]
    pub const fn session(self) -> SessionId {
        self.session
    }

    #[must_use]
    pub const fn sequence(self) -> u32 {
        self.sequence.get()
    }
}

impl fmt::Display for ElementRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "e{}", self.sequence)
    }
}

#[cfg(test)]
mod tests {
    use super::{ElementRef, SessionId, StateId};

    #[test]
    fn scoped_ids_do_not_alias_across_sessions() {
        let first = SessionId::new(1).unwrap();
        let second = SessionId::new(2).unwrap();

        assert_ne!(
            StateId::new(first, 1).unwrap(),
            StateId::new(second, 1).unwrap()
        );
        assert_ne!(
            ElementRef::new(first, 1).unwrap(),
            ElementRef::new(second, 1).unwrap()
        );
    }

    #[test]
    fn compact_reference_format_is_session_local() {
        let session = SessionId::new(9).unwrap();
        assert_eq!(ElementRef::new(session, 42).unwrap().to_string(), "e42");
    }
}
