use crate::{
    BoundedText, CapabilityReport, CollectionLimit, EngineIdentity, MeasurementSet, NonEmptyText,
    ObservationTokenBudget, PageIdentity, ResetReason, SemanticUnit, SessionId, StateId,
    ValidationError, ValidationIssue,
};

const MAX_GOAL_BYTES: usize = 4_096;
const MAX_SUMMARY_BYTES: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationRequest {
    session: SessionId,
    goal: Option<NonEmptyText<MAX_GOAL_BYTES>>,
    max_tokens: Option<ObservationTokenBudget>,
    since_state: Option<StateId>,
}

impl ObservationRequest {
    #[must_use]
    pub const fn new(session: SessionId) -> Self {
        Self {
            session,
            goal: None,
            max_tokens: None,
            since_state: None,
        }
    }

    pub fn with_goal(mut self, goal: impl Into<String>) -> Result<Self, ValidationError> {
        self.goal = Some(NonEmptyText::new(goal, "observation_goal")?);
        Ok(self)
    }

    #[must_use]
    pub const fn with_max_tokens(mut self, max_tokens: ObservationTokenBudget) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn since_state(mut self, state: StateId) -> Result<Self, ValidationError> {
        ensure_session("since_state", self.session, state)?;
        self.since_state = Some(state);
        Ok(self)
    }

    #[must_use]
    pub const fn session(&self) -> SessionId {
        self.session
    }

    #[must_use]
    pub fn goal(&self) -> Option<&str> {
        self.goal.as_ref().map(NonEmptyText::as_str)
    }

    #[must_use]
    pub const fn max_tokens(&self) -> Option<ObservationTokenBudget> {
        self.max_tokens
    }

    #[must_use]
    pub const fn requested_base(&self) -> Option<StateId> {
        self.since_state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FullObservationReason {
    Initial,
    Requested,
    NoBaseRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObservationBasis {
    Full(FullObservationReason),
    Incremental {
        base: StateId,
    },
    Reset {
        requested_base: StateId,
        reason: ResetReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObservationChanges {
    NotRequested,
    NotComputed {
        base: StateId,
    },
    Reset {
        requested_base: StateId,
        reason: ResetReason,
    },
}

impl ObservationBasis {
    fn validate_session(self, state: StateId) -> Result<(), ValidationError> {
        let base = match self {
            Self::Full(_) => return Ok(()),
            Self::Incremental { base } => base,
            Self::Reset { requested_base, .. } => requested_base,
        };
        ensure_session("observation_base", state.session(), base)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum OmissionCategory {
    Budget,
    Irrelevant,
    Unsupported,
    Sensitive,
    Duplicate,
    Other,
}

impl OmissionCategory {
    pub const COUNT: usize = 6;
    pub const ALL: [Self; Self::COUNT] = [
        Self::Budget,
        Self::Irrelevant,
        Self::Unsupported,
        Self::Sensitive,
        Self::Duplicate,
        Self::Other,
    ];

    const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmissionSummary {
    counts: [u64; OmissionCategory::COUNT],
}

impl OmissionSummary {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            counts: [0; OmissionCategory::COUNT],
        }
    }

    pub fn record(
        mut self,
        category: OmissionCategory,
        count: u64,
    ) -> Result<Self, ValidationError> {
        if count == 0 {
            return Err(ValidationError::new(
                "omission_count",
                ValidationIssue::Zero,
            ));
        }
        self.counts[category.index()] = self.counts[category.index()]
            .checked_add(count)
            .ok_or_else(|| {
                ValidationError::new(
                    "omission_count",
                    ValidationIssue::OutOfRange {
                        min: 1,
                        max: u64::MAX,
                        actual: u64::MAX,
                    },
                )
            })?;
        Ok(self)
    }

    #[must_use]
    pub const fn count(&self, category: OmissionCategory) -> u64 {
        self.counts[category.index()]
    }

    pub fn iter(&self) -> impl Iterator<Item = (OmissionCategory, u64)> + '_ {
        OmissionCategory::ALL
            .into_iter()
            .map(|category| (category, self.count(category)))
    }
}

impl Default for OmissionSummary {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    state: StateId,
    page: PageIdentity,
    engine: EngineIdentity,
    capabilities: CapabilityReport,
    basis: ObservationBasis,
    changes: ObservationChanges,
    unit_limit: CollectionLimit,
    summary: Option<BoundedText<MAX_SUMMARY_BYTES>>,
    units: Vec<SemanticUnit>,
    omissions: OmissionSummary,
    measurements: MeasurementSet,
}

impl Observation {
    pub fn new(
        state: StateId,
        page: PageIdentity,
        engine: EngineIdentity,
        capabilities: CapabilityReport,
        basis: ObservationBasis,
        unit_limit: CollectionLimit,
    ) -> Result<Self, ValidationError> {
        if page.id().session() != state.session() {
            return Err(ValidationError::new(
                "observation_page",
                ValidationIssue::SessionMismatch {
                    expected: state.session().get(),
                    actual: page.id().session().get(),
                },
            ));
        }
        if capabilities.engine() != &engine {
            return Err(ValidationError::new(
                "observation_capabilities",
                ValidationIssue::InvalidFormat,
            ));
        }
        basis.validate_session(state)?;

        let changes = match basis {
            ObservationBasis::Full(_) => ObservationChanges::NotRequested,
            ObservationBasis::Incremental { base } => ObservationChanges::NotComputed { base },
            ObservationBasis::Reset {
                requested_base,
                reason,
            } => ObservationChanges::Reset {
                requested_base,
                reason,
            },
        };

        Ok(Self {
            state,
            page,
            engine,
            capabilities,
            basis,
            changes,
            unit_limit,
            summary: None,
            units: Vec::new(),
            omissions: OmissionSummary::new(),
            measurements: MeasurementSet::default(),
        })
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Result<Self, ValidationError> {
        self.summary = Some(BoundedText::new(summary, "observation_summary")?);
        Ok(self)
    }

    pub fn with_unit(mut self, unit: SemanticUnit) -> Result<Self, ValidationError> {
        self = self.with_units(std::iter::once(unit))?;
        Ok(self)
    }

    pub fn with_units(
        mut self,
        units: impl IntoIterator<Item = SemanticUnit>,
    ) -> Result<Self, ValidationError> {
        let units = units.into_iter().collect::<Vec<_>>();
        if self.units.len().saturating_add(units.len()) > self.unit_limit.get() as usize {
            return Err(ValidationError::new(
                "semantic_units",
                ValidationIssue::OutOfRange {
                    min: 0,
                    max: self.unit_limit.get(),
                    actual: self.units.len().saturating_add(units.len()) as u64,
                },
            ));
        }
        if let Some(unit) = units
            .iter()
            .find(|unit| unit.reference().session() != self.state.session())
        {
            return Err(ValidationError::new(
                "semantic_unit",
                ValidationIssue::SessionMismatch {
                    expected: self.state.session().get(),
                    actual: unit.reference().session().get(),
                },
            ));
        }
        self.units.extend(units);
        self.units.sort_unstable_by_key(SemanticUnit::reference);
        if self
            .units
            .windows(2)
            .any(|pair| pair[0].reference() == pair[1].reference())
        {
            return Err(ValidationError::new(
                "semantic_unit",
                ValidationIssue::Duplicate,
            ));
        }
        Ok(self)
    }

    #[must_use]
    pub fn with_omissions(mut self, omissions: OmissionSummary) -> Self {
        self.omissions = omissions;
        self
    }

    #[must_use]
    pub fn with_measurements(mut self, measurements: MeasurementSet) -> Self {
        self.measurements = measurements;
        self
    }

    #[must_use]
    pub const fn state(&self) -> StateId {
        self.state
    }

    #[must_use]
    pub const fn page(&self) -> &PageIdentity {
        &self.page
    }

    #[must_use]
    pub const fn engine(&self) -> &EngineIdentity {
        &self.engine
    }

    #[must_use]
    pub const fn capabilities(&self) -> &CapabilityReport {
        &self.capabilities
    }

    #[must_use]
    pub const fn basis(&self) -> ObservationBasis {
        self.basis
    }

    #[must_use]
    pub const fn changes(&self) -> ObservationChanges {
        self.changes
    }

    #[must_use]
    pub const fn unit_limit(&self) -> CollectionLimit {
        self.unit_limit
    }

    #[must_use]
    pub fn summary(&self) -> Option<&str> {
        self.summary.as_ref().map(BoundedText::as_str)
    }

    #[must_use]
    pub fn units(&self) -> &[SemanticUnit] {
        &self.units
    }

    #[must_use]
    pub const fn omissions(&self) -> &OmissionSummary {
        &self.omissions
    }

    #[must_use]
    pub const fn measurements(&self) -> &MeasurementSet {
        &self.measurements
    }
}

fn ensure_session(
    field: &'static str,
    expected: SessionId,
    actual: StateId,
) -> Result<(), ValidationError> {
    if expected != actual.session() {
        return Err(ValidationError::new(
            field,
            ValidationIssue::SessionMismatch {
                expected: expected.get(),
                actual: actual.session().get(),
            },
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        AbsoluteUrl, CapabilityReport, ElementRef, EngineIdentity, EngineKind, PageId, Provenance,
        SemanticRole, SemanticUnit, SessionId, StateId, UnsupportedReason,
    };

    use super::{FullObservationReason, Observation, ObservationBasis};

    fn observation(session: SessionId) -> Observation {
        let engine = EngineIdentity::new("native-static", "0", EngineKind::NativeStatic).unwrap();
        let capabilities =
            CapabilityReport::unsupported_all(engine.clone(), UnsupportedReason::NotImplemented);
        Observation::new(
            StateId::new(session, 1).unwrap(),
            crate::PageIdentity::new(
                PageId::new(session, 1).unwrap(),
                AbsoluteUrl::new("https://example.test").unwrap(),
            ),
            engine,
            capabilities,
            ObservationBasis::Full(FullObservationReason::Initial),
            crate::CollectionLimit::new(100, "unit_limit").unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn units_are_canonicalized_by_reference() {
        let session = SessionId::new(1).unwrap();
        let first = SemanticUnit::new(
            ElementRef::new(session, 2).unwrap(),
            SemanticRole::Text,
            Provenance::UntrustedWebContent,
        );
        let second = SemanticUnit::new(
            ElementRef::new(session, 1).unwrap(),
            SemanticRole::Heading,
            Provenance::UntrustedWebContent,
        );
        let result = observation(session)
            .with_unit(first)
            .unwrap()
            .with_unit(second)
            .unwrap();

        assert_eq!(result.units()[0].reference().sequence(), 1);
        assert_eq!(result.units()[1].reference().sequence(), 2);
    }

    #[test]
    fn duplicate_and_cross_session_units_fail_closed() {
        let session = SessionId::new(1).unwrap();
        let reference = ElementRef::new(session, 1).unwrap();
        let unit = SemanticUnit::new(
            reference,
            SemanticRole::Text,
            Provenance::UntrustedWebContent,
        );
        assert!(
            observation(session)
                .with_unit(unit.clone())
                .unwrap()
                .with_unit(unit)
                .is_err()
        );

        let other = SessionId::new(2).unwrap();
        let foreign = SemanticUnit::new(
            ElementRef::new(other, 1).unwrap(),
            SemanticRole::Text,
            Provenance::UntrustedWebContent,
        );
        assert!(observation(session).with_unit(foreign).is_err());
    }

    #[test]
    fn configured_unit_limit_is_enforced() {
        let session = SessionId::new(1).unwrap();
        let mut subject = observation(session);
        subject.unit_limit = crate::CollectionLimit::new(1, "unit_limit").unwrap();
        let first = SemanticUnit::new(
            ElementRef::new(session, 1).unwrap(),
            SemanticRole::Text,
            Provenance::UntrustedWebContent,
        );
        let second = SemanticUnit::new(
            ElementRef::new(session, 2).unwrap(),
            SemanticRole::Text,
            Provenance::UntrustedWebContent,
        );

        assert!(subject.with_unit(first).unwrap().with_unit(second).is_err());
    }
}
