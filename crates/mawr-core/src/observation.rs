use std::collections::BTreeSet;

use crate::{
    BoundedText, CapabilityReport, CollectionLimit, ElementRef, EngineIdentity, MeasurementSet,
    NonEmptyText, ObservationTokenBudget, PageIdentity, ResetReason, SemanticUnit, SessionId,
    StateId, ValidationError, ValidationIssue,
};

const MAX_GOAL_BYTES: usize = 4_096;
pub const MAX_SUMMARY_BYTES: usize = 1_024;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationChanges {
    NotRequested,
    NotComputed {
        base: StateId,
    },
    Computed(SemanticChanges),
    Reset {
        requested_base: StateId,
        reason: ResetReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticChanges {
    base: StateId,
    target: StateId,
    added: Vec<ElementRef>,
    updated: Vec<ElementRef>,
    removed: Vec<ElementRef>,
    summary_changed: bool,
    order_changed: bool,
}

impl SemanticChanges {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base: StateId,
        target: StateId,
        added: Vec<ElementRef>,
        updated: Vec<ElementRef>,
        removed: Vec<ElementRef>,
        summary_changed: bool,
        order_changed: bool,
        change_limit: CollectionLimit,
    ) -> Result<Self, ValidationError> {
        ensure_session("semantic_changes_target", base.session(), target)?;
        if base.sequence() > target.sequence() {
            return Err(ValidationError::new(
                "semantic_changes_state",
                ValidationIssue::InvalidTransition,
            ));
        }
        let added = validated_references("semantic_changes_added", target, added)?;
        let updated = validated_references("semantic_changes_updated", target, updated)?;
        let removed = validated_references("semantic_changes_removed", target, removed)?;
        let entry_count = added
            .len()
            .saturating_add(updated.len())
            .saturating_add(removed.len());
        if entry_count > change_limit.get() as usize {
            return Err(ValidationError::new(
                "semantic_changes",
                ValidationIssue::OutOfRange {
                    min: 0,
                    max: change_limit.get(),
                    actual: entry_count as u64,
                },
            ));
        }
        let mut all = BTreeSet::new();
        if added
            .iter()
            .chain(&updated)
            .chain(&removed)
            .any(|reference| !all.insert(*reference))
        {
            return Err(ValidationError::new(
                "semantic_changes_reference",
                ValidationIssue::Duplicate,
            ));
        }
        Ok(Self {
            base,
            target,
            added,
            updated,
            removed,
            summary_changed,
            order_changed,
        })
    }

    #[must_use]
    pub const fn base(&self) -> StateId {
        self.base
    }

    #[must_use]
    pub const fn target(&self) -> StateId {
        self.target
    }

    #[must_use]
    pub fn added(&self) -> &[ElementRef] {
        &self.added
    }

    #[must_use]
    pub fn updated(&self) -> &[ElementRef] {
        &self.updated
    }

    #[must_use]
    pub fn removed(&self) -> &[ElementRef] {
        &self.removed
    }

    #[must_use]
    pub const fn summary_changed(&self) -> bool {
        self.summary_changed
    }

    #[must_use]
    pub const fn order_changed(&self) -> bool {
        self.order_changed
    }

    pub fn changed_references(&self) -> impl Iterator<Item = ElementRef> + '_ {
        self.added.iter().chain(&self.updated).copied()
    }

    #[must_use]
    pub const fn unit_change_count(&self) -> usize {
        self.added.len() + self.updated.len() + self.removed.len()
    }
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
    semantic_order: Vec<ElementRef>,
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
            semantic_order: Vec::new(),
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
        let appended_order = units
            .iter()
            .map(SemanticUnit::reference)
            .collect::<Vec<_>>();
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
        self.semantic_order.extend(appended_order);
        Ok(self)
    }

    pub fn with_computed_changes(
        mut self,
        changes: SemanticChanges,
        units: Vec<SemanticUnit>,
        summary: Option<String>,
        target_order: Option<Vec<ElementRef>>,
    ) -> Result<Self, ValidationError> {
        let ObservationBasis::Incremental { base } = self.basis else {
            return Err(ValidationError::new(
                "observation_changes",
                ValidationIssue::InvalidTransition,
            ));
        };
        if changes.base() != base || changes.target() != self.state {
            return Err(ValidationError::new(
                "observation_changes_state",
                ValidationIssue::InvalidTransition,
            ));
        }
        validate_units(self.state, self.unit_limit, &units)?;
        let unit_references = units
            .iter()
            .map(SemanticUnit::reference)
            .collect::<BTreeSet<_>>();
        let changed_references = changes.changed_references().collect::<BTreeSet<_>>();
        if unit_references != changed_references {
            return Err(ValidationError::new(
                "observation_changed_units",
                ValidationIssue::InvalidFormat,
            ));
        }
        if !changes.summary_changed() && summary.is_some() {
            return Err(ValidationError::new(
                "observation_changed_summary",
                ValidationIssue::InvalidFormat,
            ));
        }
        let summary = if changes.summary_changed() {
            summary
                .map(|summary| BoundedText::new(summary, "observation_summary"))
                .transpose()?
        } else {
            None
        };
        let semantic_order = match (changes.order_changed(), target_order) {
            (false, None) => Vec::new(),
            (true, Some(order)) => {
                validate_order(self.state, self.unit_limit, &order)?;
                let order_set = order.iter().copied().collect::<BTreeSet<_>>();
                if changes
                    .changed_references()
                    .any(|reference| !order_set.contains(&reference))
                    || changes
                        .removed()
                        .iter()
                        .any(|reference| order_set.contains(reference))
                {
                    return Err(ValidationError::new(
                        "observation_changed_order",
                        ValidationIssue::InvalidFormat,
                    ));
                }
                order
            }
            _ => {
                return Err(ValidationError::new(
                    "observation_changed_order",
                    ValidationIssue::InvalidFormat,
                ));
            }
        };
        let mut units = units;
        units.sort_unstable_by_key(SemanticUnit::reference);
        self.changes = ObservationChanges::Computed(changes);
        self.summary = summary;
        self.units = units;
        self.semantic_order = semantic_order;
        Ok(self)
    }

    pub fn with_selected_units(
        mut self,
        references: impl IntoIterator<Item = crate::ElementRef>,
        omissions: OmissionSummary,
    ) -> Result<Self, ValidationError> {
        if matches!(self.changes, ObservationChanges::Computed(_)) {
            return Err(ValidationError::new(
                "selected_incremental_observation",
                ValidationIssue::InvalidTransition,
            ));
        }
        let references = references.into_iter().collect::<Vec<_>>();
        let selected = references.iter().copied().collect::<BTreeSet<_>>();
        if selected.len() != references.len() {
            return Err(ValidationError::new(
                "selected_semantic_unit",
                ValidationIssue::Duplicate,
            ));
        }
        if let Some(reference) = selected
            .iter()
            .find(|reference| reference.session() != self.state.session())
        {
            return Err(ValidationError::new(
                "selected_semantic_unit",
                ValidationIssue::SessionMismatch {
                    expected: self.state.session().get(),
                    actual: reference.session().get(),
                },
            ));
        }
        if selected.iter().any(|reference| {
            self.units
                .binary_search_by_key(reference, SemanticUnit::reference)
                .is_err()
        }) {
            return Err(ValidationError::new(
                "selected_semantic_unit",
                ValidationIssue::InvalidFormat,
            ));
        }
        self.units
            .retain(|unit| selected.contains(&unit.reference()));
        self.semantic_order
            .retain(|reference| selected.contains(reference));
        self.omissions = omissions;
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
    pub const fn changes(&self) -> &ObservationChanges {
        &self.changes
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
    pub fn semantic_order(&self) -> &[ElementRef] {
        &self.semantic_order
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

fn validated_references(
    field: &'static str,
    state: StateId,
    mut references: Vec<ElementRef>,
) -> Result<Vec<ElementRef>, ValidationError> {
    if let Some(reference) = references
        .iter()
        .find(|reference| reference.session() != state.session())
    {
        return Err(ValidationError::new(
            field,
            ValidationIssue::SessionMismatch {
                expected: state.session().get(),
                actual: reference.session().get(),
            },
        ));
    }
    references.sort_unstable();
    if references.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ValidationError::new(field, ValidationIssue::Duplicate));
    }
    Ok(references)
}

fn validate_units(
    state: StateId,
    limit: CollectionLimit,
    units: &[SemanticUnit],
) -> Result<(), ValidationError> {
    if units.len() > limit.get() as usize {
        return Err(ValidationError::new(
            "semantic_units",
            ValidationIssue::OutOfRange {
                min: 0,
                max: limit.get(),
                actual: units.len() as u64,
            },
        ));
    }
    let references = units
        .iter()
        .map(SemanticUnit::reference)
        .collect::<Vec<_>>();
    validated_references("semantic_unit", state, references).map(|_| ())
}

fn validate_order(
    state: StateId,
    limit: CollectionLimit,
    order: &[ElementRef],
) -> Result<(), ValidationError> {
    if order.len() > limit.get() as usize {
        return Err(ValidationError::new(
            "semantic_order",
            ValidationIssue::OutOfRange {
                min: 0,
                max: limit.get(),
                actual: order.len() as u64,
            },
        ));
    }
    validated_references("semantic_order", state, order.to_vec()).map(|_| ())
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

    use super::{FullObservationReason, Observation, ObservationBasis, SemanticChanges};

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

    #[test]
    fn selected_units_must_be_unique_owned_members() {
        let session = SessionId::new(1).unwrap();
        let first = ElementRef::new(session, 1).unwrap();
        let second = ElementRef::new(session, 2).unwrap();
        let subject = observation(session)
            .with_units([
                SemanticUnit::new(first, SemanticRole::Page, Provenance::UntrustedWebContent),
                SemanticUnit::new(
                    second,
                    SemanticRole::Button,
                    Provenance::UntrustedWebContent,
                ),
            ])
            .unwrap();
        let selected = subject
            .clone()
            .with_selected_units([second], super::OmissionSummary::new())
            .unwrap();
        assert_eq!(selected.units().len(), 1);
        assert_eq!(selected.units()[0].reference(), second);
        assert!(
            subject
                .clone()
                .with_selected_units([first, first], super::OmissionSummary::new())
                .is_err()
        );
        assert!(
            subject
                .clone()
                .with_selected_units(
                    [ElementRef::new(session, 99).unwrap()],
                    super::OmissionSummary::new(),
                )
                .is_err()
        );
        let foreign = SessionId::new(2).unwrap();
        assert!(
            subject
                .with_selected_units(
                    [ElementRef::new(foreign, 1).unwrap()],
                    super::OmissionSummary::new(),
                )
                .is_err()
        );
    }

    #[test]
    fn semantic_changes_are_bounded_disjoint_and_session_scoped() {
        let session = SessionId::new(3).unwrap();
        let base = StateId::new(session, 1).unwrap();
        let target = StateId::new(session, 2).unwrap();
        let added = ElementRef::new(session, 1).unwrap();
        let updated = ElementRef::new(session, 2).unwrap();
        let removed = ElementRef::new(session, 3).unwrap();
        let changes = SemanticChanges::new(
            base,
            target,
            vec![added],
            vec![updated],
            vec![removed],
            true,
            true,
            crate::CollectionLimit::new(3, "change_limit").unwrap(),
        )
        .unwrap();
        assert_eq!(changes.unit_change_count(), 3);
        assert_eq!(
            changes.changed_references().collect::<Vec<_>>(),
            vec![added, updated]
        );

        assert!(
            SemanticChanges::new(
                base,
                target,
                vec![added],
                vec![added],
                Vec::new(),
                false,
                false,
                crate::CollectionLimit::new(3, "change_limit").unwrap(),
            )
            .is_err()
        );
        assert!(
            SemanticChanges::new(
                base,
                target,
                vec![added, updated],
                vec![removed],
                Vec::new(),
                false,
                false,
                crate::CollectionLimit::new(2, "change_limit").unwrap(),
            )
            .is_err()
        );
        let foreign = SessionId::new(4).unwrap();
        assert!(
            SemanticChanges::new(
                base,
                target,
                vec![ElementRef::new(foreign, 1).unwrap()],
                Vec::new(),
                Vec::new(),
                false,
                false,
                crate::CollectionLimit::new(3, "change_limit").unwrap(),
            )
            .is_err()
        );
    }
}
