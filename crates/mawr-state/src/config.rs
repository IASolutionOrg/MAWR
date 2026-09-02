use std::fmt;

const MAX_RETAINED_STATES: usize = 1_024;
const MAX_RETAINED_UNITS: usize = 10_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    Zero { field: &'static str },
    ExceedsMaximum { field: &'static str, maximum: usize },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero { field } => write!(formatter, "{field} must be non-zero"),
            Self::ExceedsMaximum { field, maximum } => {
                write!(formatter, "{field} exceeds maximum {maximum}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateStoreConfig {
    retained_states: usize,
    retained_units: usize,
}

impl StateStoreConfig {
    pub fn with_retained_states(mut self, value: usize) -> Result<Self, ConfigError> {
        self.retained_states = validate(value, "retained_states", MAX_RETAINED_STATES)?;
        Ok(self)
    }

    pub fn with_retained_units(mut self, value: usize) -> Result<Self, ConfigError> {
        self.retained_units = validate(value, "retained_units", MAX_RETAINED_UNITS)?;
        Ok(self)
    }

    #[must_use]
    pub const fn retained_states(self) -> usize {
        self.retained_states
    }

    #[must_use]
    pub const fn retained_units(self) -> usize {
        self.retained_units
    }
}

impl Default for StateStoreConfig {
    fn default() -> Self {
        Self {
            retained_states: 16,
            retained_units: 500_000,
        }
    }
}

fn validate(value: usize, field: &'static str, maximum: usize) -> Result<usize, ConfigError> {
    if value == 0 {
        return Err(ConfigError::Zero { field });
    }
    if value > maximum {
        return Err(ConfigError::ExceedsMaximum { field, maximum });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_are_non_zero_and_capped() {
        assert!(StateStoreConfig::default().with_retained_states(0).is_err());
        assert!(
            StateStoreConfig::default()
                .with_retained_units(MAX_RETAINED_UNITS + 1)
                .is_err()
        );
    }
}
