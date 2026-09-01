use std::fmt;
use std::num::NonZeroU64;

const MAX_DOCUMENT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_DOM_NODES: u64 = 1_000_000;
const MAX_DOM_DEPTH: u64 = 4_096;
const MAX_ATTRIBUTES: u64 = 256;
const MAX_TEXT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SEMANTIC_UNITS: u64 = 250_000;
const MAX_RELATIONSHIPS_PER_UNIT: u64 = 128;
const MAX_NOTICES: u64 = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    Zero,
    AboveMaximum { maximum: u64, actual: u64 },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("an extraction limit cannot be zero"),
            Self::AboveMaximum { maximum, actual } => {
                write!(
                    formatter,
                    "extraction limit {actual} exceeds maximum {maximum}"
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionLimits {
    document_bytes: NonZeroU64,
    dom_nodes: NonZeroU64,
    dom_depth: NonZeroU64,
    attributes_per_element: NonZeroU64,
    document_text_bytes: NonZeroU64,
    semantic_units: NonZeroU64,
    relationships_per_unit: NonZeroU64,
    notices: NonZeroU64,
}

impl ExtractionLimits {
    pub fn with_document_bytes(mut self, value: u64) -> Result<Self, ConfigError> {
        self.document_bytes = bounded(value, MAX_DOCUMENT_BYTES)?;
        Ok(self)
    }

    pub fn with_dom_nodes(mut self, value: u64) -> Result<Self, ConfigError> {
        self.dom_nodes = bounded(value, MAX_DOM_NODES)?;
        Ok(self)
    }

    pub fn with_dom_depth(mut self, value: u64) -> Result<Self, ConfigError> {
        self.dom_depth = bounded(value, MAX_DOM_DEPTH)?;
        Ok(self)
    }

    pub fn with_attributes_per_element(mut self, value: u64) -> Result<Self, ConfigError> {
        self.attributes_per_element = bounded(value, MAX_ATTRIBUTES)?;
        Ok(self)
    }

    pub fn with_document_text_bytes(mut self, value: u64) -> Result<Self, ConfigError> {
        self.document_text_bytes = bounded(value, MAX_TEXT_BYTES)?;
        Ok(self)
    }

    pub fn with_semantic_units(mut self, value: u64) -> Result<Self, ConfigError> {
        self.semantic_units = bounded(value, MAX_SEMANTIC_UNITS)?;
        Ok(self)
    }

    pub fn with_notices(mut self, value: u64) -> Result<Self, ConfigError> {
        self.notices = bounded(value, MAX_NOTICES)?;
        Ok(self)
    }

    pub fn with_relationships_per_unit(mut self, value: u64) -> Result<Self, ConfigError> {
        self.relationships_per_unit = bounded(value, MAX_RELATIONSHIPS_PER_UNIT)?;
        Ok(self)
    }

    pub(crate) const fn document_bytes(&self) -> NonZeroU64 {
        self.document_bytes
    }

    pub(crate) const fn dom_nodes(&self) -> NonZeroU64 {
        self.dom_nodes
    }

    pub(crate) const fn dom_depth(&self) -> NonZeroU64 {
        self.dom_depth
    }

    pub(crate) const fn attributes_per_element(&self) -> NonZeroU64 {
        self.attributes_per_element
    }

    pub(crate) const fn document_text_bytes(&self) -> NonZeroU64 {
        self.document_text_bytes
    }

    pub(crate) const fn semantic_units(&self) -> NonZeroU64 {
        self.semantic_units
    }

    pub(crate) const fn notices(&self) -> NonZeroU64 {
        self.notices
    }

    pub(crate) const fn relationships_per_unit(&self) -> NonZeroU64 {
        self.relationships_per_unit
    }
}

impl Default for ExtractionLimits {
    fn default() -> Self {
        Self {
            document_bytes: NonZeroU64::new(2 * 1024 * 1024).expect("constant is non-zero"),
            dom_nodes: NonZeroU64::new(50_000).expect("constant is non-zero"),
            dom_depth: NonZeroU64::new(256).expect("constant is non-zero"),
            attributes_per_element: NonZeroU64::new(128).expect("constant is non-zero"),
            document_text_bytes: NonZeroU64::new(8 * 1024 * 1024).expect("constant is non-zero"),
            semantic_units: NonZeroU64::new(20_000).expect("constant is non-zero"),
            relationships_per_unit: NonZeroU64::new(MAX_RELATIONSHIPS_PER_UNIT)
                .expect("constant is non-zero"),
            notices: NonZeroU64::new(1_024).expect("constant is non-zero"),
        }
    }
}

fn bounded(value: u64, maximum: u64) -> Result<NonZeroU64, ConfigError> {
    let value = NonZeroU64::new(value).ok_or(ConfigError::Zero)?;
    if value.get() > maximum {
        return Err(ConfigError::AboveMaximum {
            maximum,
            actual: value.get(),
        });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::ExtractionLimits;

    #[test]
    fn limits_are_nonzero_and_capped() {
        assert!(ExtractionLimits::default().with_dom_nodes(0).is_err());
        assert!(
            ExtractionLimits::default()
                .with_dom_nodes(1_000_001)
                .is_err()
        );
    }
}
