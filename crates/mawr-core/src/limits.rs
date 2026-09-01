use crate::{ValidationError, ValidationIssue};

/// A numeric value whose inclusive bounds are part of its Rust type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundedU64<const MIN: u64, const MAX: u64>(u64);

impl<const MIN: u64, const MAX: u64> BoundedU64<MIN, MAX> {
    pub fn new(value: u64, field: &'static str) -> Result<Self, ValidationError> {
        if MIN > MAX {
            return Err(ValidationError::new(
                field,
                ValidationIssue::InvalidBounds { min: MIN, max: MAX },
            ));
        }
        if !(MIN..=MAX).contains(&value) {
            return Err(ValidationError::new(
                field,
                ValidationIssue::OutOfRange {
                    min: MIN,
                    max: MAX,
                    actual: value,
                },
            ));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

pub type ObservationTokenBudget = BoundedU64<1, 1_000_000>;
pub type CollectionLimit = BoundedU64<1, 1_000_000>;

#[cfg(test)]
mod tests {
    use super::BoundedU64;

    #[test]
    fn bounded_value_acceptance_matches_the_declared_property() {
        type Subject = BoundedU64<7, 91>;

        let mut value = 0_u64;
        while value <= 100 {
            assert_eq!(
                Subject::new(value, "subject").is_ok(),
                (7..=91).contains(&value)
            );
            value += 1;
        }
    }

    #[test]
    fn invalid_const_bounds_fail_closed() {
        assert!(BoundedU64::<2, 1>::new(1, "invalid").is_err());
    }
}
