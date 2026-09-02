use mawr_core::{NonEmptyText, ValidationError};

const MAX_RANKING_VERSION_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankingWeights {
    pub goal_name_per_term: i64,
    pub goal_description_per_term: i64,
    pub goal_value_per_term: i64,
    pub interactive: i64,
    pub structural: i64,
    pub alert: i64,
    pub invalid: i64,
    pub changed: i64,
    pub context: i64,
    pub boilerplate_penalty: i64,
    pub repeated_navigation_penalty: i64,
}

impl Default for RankingWeights {
    fn default() -> Self {
        Self {
            goal_name_per_term: 120,
            goal_description_per_term: 60,
            goal_value_per_term: 50,
            interactive: 80,
            structural: 30,
            alert: 600,
            invalid: 500,
            changed: 180,
            context: 40,
            boilerplate_penalty: 70,
            repeated_navigation_penalty: 120,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankingConfig {
    version: NonEmptyText<MAX_RANKING_VERSION_BYTES>,
    weights: RankingWeights,
    minimum_score: i64,
    reserved_tokens: u64,
}

impl RankingConfig {
    pub fn new(version: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self {
            version: NonEmptyText::new(version, "ranking_version")?,
            weights: RankingWeights::default(),
            minimum_score: 1,
            reserved_tokens: 0,
        })
    }

    #[must_use]
    pub const fn with_weights(mut self, weights: RankingWeights) -> Self {
        self.weights = weights;
        self
    }

    #[must_use]
    pub const fn with_minimum_score(mut self, minimum_score: i64) -> Self {
        self.minimum_score = minimum_score;
        self
    }

    #[must_use]
    pub const fn with_reserved_tokens(mut self, reserved_tokens: u64) -> Self {
        self.reserved_tokens = reserved_tokens;
        self
    }

    #[must_use]
    pub fn version(&self) -> &str {
        self.version.as_str()
    }

    #[must_use]
    pub const fn weights(&self) -> RankingWeights {
        self.weights
    }

    #[must_use]
    pub const fn minimum_score(&self) -> i64 {
        self.minimum_score
    }

    #[must_use]
    pub const fn reserved_tokens(&self) -> u64 {
        self.reserved_tokens
    }
}

impl Default for RankingConfig {
    fn default() -> Self {
        Self::new("mawr-relevance-v1").expect("default ranking version is valid")
    }
}
