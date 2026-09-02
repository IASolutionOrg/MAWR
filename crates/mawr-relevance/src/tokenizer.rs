use mawr_core::{NonEmptyText, ValidationError};

const MAX_TOKENIZER_NAME_BYTES: usize = 48;
const MAX_TOKENIZER_VERSION_BYTES: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TokenCountQuality {
    Exact,
    Estimated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenizerMetadata {
    name: NonEmptyText<MAX_TOKENIZER_NAME_BYTES>,
    version: NonEmptyText<MAX_TOKENIZER_VERSION_BYTES>,
    quality: TokenCountQuality,
}

impl TokenizerMetadata {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        quality: TokenCountQuality,
    ) -> Result<Self, ValidationError> {
        Ok(Self {
            name: NonEmptyText::new(name, "tokenizer_name")?,
            version: NonEmptyText::new(version, "tokenizer_version")?,
            quality,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    #[must_use]
    pub fn version(&self) -> &str {
        self.version.as_str()
    }

    #[must_use]
    pub const fn quality(&self) -> TokenCountQuality {
        self.quality
    }
}

pub trait TokenCounter {
    fn metadata(&self) -> &TokenizerMetadata;

    /// Counts one independently framed diagnostic projection.
    ///
    /// M6 adds fragment counts, so exact implementations must be additive for
    /// independently framed inputs and return at least one token for every
    /// non-empty fragment. M10 remains responsible for measuring the eventual
    /// transport encoding as a whole.
    fn count_tokens(&self, input: &str) -> u64;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utf8ByteEstimator {
    metadata: TokenizerMetadata,
}

impl Utf8ByteEstimator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            metadata: TokenizerMetadata::new("utf8-bytes-div-4", "1", TokenCountQuality::Estimated)
                .expect("built-in tokenizer metadata is valid"),
        }
    }
}

impl Default for Utf8ByteEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCounter for Utf8ByteEstimator {
    fn metadata(&self) -> &TokenizerMetadata {
        &self.metadata
    }

    fn count_tokens(&self, input: &str) -> u64 {
        if input.is_empty() {
            return 0;
        }
        u64::try_from(input.len())
            .unwrap_or(u64::MAX)
            .saturating_add(3)
            / 4
    }
}
