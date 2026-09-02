use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use mawr_core::{
    CollectionLimit, ElementRef, Observation, ObservationBasis, ObservationChanges,
    OperationFailure, PageIdentity, SemanticChanges, SemanticUnit, StateId, ValidationIssue,
};
use mawr_state::StoredState;

use crate::builder::{convert_unit, page_summary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticDiffDiagnostics {
    base_unit_count: usize,
    target_unit_count: usize,
    emitted_unit_count: usize,
    added_count: usize,
    updated_count: usize,
    removed_count: usize,
    summary_changed: bool,
    order_changed: bool,
    entry_count: usize,
    construction_latency_micros: u64,
}

impl SemanticDiffDiagnostics {
    #[must_use]
    pub const fn base_unit_count(&self) -> usize {
        self.base_unit_count
    }

    #[must_use]
    pub const fn target_unit_count(&self) -> usize {
        self.target_unit_count
    }

    #[must_use]
    pub const fn emitted_unit_count(&self) -> usize {
        self.emitted_unit_count
    }

    #[must_use]
    pub const fn added_count(&self) -> usize {
        self.added_count
    }

    #[must_use]
    pub const fn updated_count(&self) -> usize {
        self.updated_count
    }

    #[must_use]
    pub const fn removed_count(&self) -> usize {
        self.removed_count
    }

    #[must_use]
    pub const fn summary_changed(&self) -> bool {
        self.summary_changed
    }

    #[must_use]
    pub const fn order_changed(&self) -> bool {
        self.order_changed
    }

    #[must_use]
    pub const fn entry_count(&self) -> usize {
        self.entry_count
    }

    #[must_use]
    pub const fn construction_latency_micros(&self) -> u64 {
        self.construction_latency_micros
    }
}

pub(crate) struct ComputedDiff {
    pub(crate) changes: SemanticChanges,
    pub(crate) units: Vec<SemanticUnit>,
    pub(crate) summary: Option<String>,
    pub(crate) target_order: Option<Vec<ElementRef>>,
    pub(crate) diagnostics: SemanticDiffDiagnostics,
}

pub(crate) fn compute_diff(
    base: &StoredState,
    target: &StoredState,
    change_limit: CollectionLimit,
) -> Result<Option<ComputedDiff>, OperationFailure> {
    let started = Instant::now();
    let limit = change_limit.get() as usize;
    if base.units().len() > limit || target.units().len() > limit {
        return Ok(None);
    }

    let base_units = base
        .units()
        .iter()
        .map(convert_unit)
        .collect::<Result<Vec<_>, _>>()?;
    let target_units = target
        .units()
        .iter()
        .map(convert_unit)
        .collect::<Result<Vec<_>, _>>()?;
    let base_by_reference = base_units
        .iter()
        .map(|unit| (unit.reference(), unit))
        .collect::<BTreeMap<_, _>>();
    let target_by_reference = target_units
        .iter()
        .map(|unit| (unit.reference(), unit))
        .collect::<BTreeMap<_, _>>();

    let mut added = Vec::new();
    let mut updated = Vec::new();
    let mut emitted = Vec::new();
    for unit in &target_units {
        match base_by_reference.get(&unit.reference()) {
            None => {
                added.push(unit.reference());
                emitted.push(unit.clone());
            }
            Some(previous) if *previous != unit => {
                updated.push(unit.reference());
                emitted.push(unit.clone());
            }
            Some(_) => {}
        }
    }
    let removed = base_units
        .iter()
        .filter(|unit| !target_by_reference.contains_key(&unit.reference()))
        .map(SemanticUnit::reference)
        .collect::<Vec<_>>();
    let base_order = base
        .units()
        .iter()
        .map(|unit| unit.reference())
        .collect::<Vec<_>>();
    let target_order = target
        .units()
        .iter()
        .map(|unit| unit.reference())
        .collect::<Vec<_>>();
    let order_changed = base_order != target_order;
    let base_summary = page_summary(base);
    let target_summary = page_summary(target);
    let summary_changed = base_summary != target_summary;
    let entry_count = added
        .len()
        .saturating_add(updated.len())
        .saturating_add(removed.len())
        .saturating_add(if order_changed { target_order.len() } else { 0 })
        .saturating_add(usize::from(summary_changed));
    if entry_count > limit {
        return Ok(None);
    }

    let changes = SemanticChanges::new(
        base.id(),
        target.id(),
        added,
        updated,
        removed,
        summary_changed,
        order_changed,
        change_limit,
    )
    .map_err(OperationFailure::InvalidInput)?;
    let diagnostics = SemanticDiffDiagnostics {
        base_unit_count: base_units.len(),
        target_unit_count: target_units.len(),
        emitted_unit_count: emitted.len(),
        added_count: changes.added().len(),
        updated_count: changes.updated().len(),
        removed_count: changes.removed().len(),
        summary_changed,
        order_changed,
        entry_count,
        construction_latency_micros: u64::try_from(started.elapsed().as_micros())
            .unwrap_or(u64::MAX),
    };
    Ok(Some(ComputedDiff {
        changes,
        units: emitted,
        summary: summary_changed.then_some(target_summary),
        target_order: order_changed.then_some(target_order),
        diagnostics,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticSnapshot {
    state: StateId,
    page: PageIdentity,
    summary: Option<String>,
    units: Vec<SemanticUnit>,
    order: Vec<ElementRef>,
}

impl SemanticSnapshot {
    pub fn from_full(observation: &Observation) -> Result<Self, OperationFailure> {
        if matches!(
            observation.changes(),
            ObservationChanges::Computed(_) | ObservationChanges::NotComputed { .. }
        ) {
            return Err(invalid_transition("snapshot_requires_full_observation"));
        }
        validate_exact_order(observation.units(), observation.semantic_order())?;
        Ok(Self {
            state: observation.state(),
            page: observation.page().clone(),
            summary: observation.summary().map(str::to_owned),
            units: observation.units().to_vec(),
            order: observation.semantic_order().to_vec(),
        })
    }

    pub fn apply(&self, observation: &Observation) -> Result<Self, OperationFailure> {
        let ObservationBasis::Incremental { base } = observation.basis() else {
            return Err(invalid_transition(
                "snapshot_requires_incremental_observation",
            ));
        };
        let ObservationChanges::Computed(changes) = observation.changes() else {
            return Err(invalid_transition("snapshot_requires_computed_changes"));
        };
        if base != self.state
            || changes.base() != self.state
            || changes.target() != observation.state()
            || observation.page() != &self.page
        {
            return Err(invalid_transition("snapshot_diff_base"));
        }

        let emitted = observation
            .units()
            .iter()
            .map(|unit| (unit.reference(), unit.clone()))
            .collect::<BTreeMap<_, _>>();
        let expected_emitted = changes.changed_references().collect::<BTreeSet<_>>();
        if emitted.keys().copied().collect::<BTreeSet<_>>() != expected_emitted {
            return Err(invalid_transition("snapshot_diff_units"));
        }
        let mut units = self
            .units
            .iter()
            .cloned()
            .map(|unit| (unit.reference(), unit))
            .collect::<BTreeMap<_, _>>();
        for reference in changes.removed() {
            if units.remove(reference).is_none() {
                return Err(invalid_transition("snapshot_missing_removal"));
            }
        }
        for reference in changes.updated() {
            if !units.contains_key(reference) {
                return Err(invalid_transition("snapshot_missing_update"));
            }
            units.insert(
                *reference,
                emitted
                    .get(reference)
                    .expect("computed changes require every updated unit")
                    .clone(),
            );
        }
        for reference in changes.added() {
            if units.contains_key(reference) {
                return Err(invalid_transition("snapshot_duplicate_addition"));
            }
            units.insert(
                *reference,
                emitted
                    .get(reference)
                    .expect("computed changes require every added unit")
                    .clone(),
            );
        }
        let order = if changes.order_changed() {
            observation.semantic_order().to_vec()
        } else {
            self.order.clone()
        };
        let ordered_set = order.iter().copied().collect::<BTreeSet<_>>();
        if order.len() != ordered_set.len()
            || ordered_set != units.keys().copied().collect::<BTreeSet<_>>()
        {
            return Err(invalid_transition("snapshot_target_order"));
        }
        let summary = if changes.summary_changed() {
            observation.summary().map(str::to_owned)
        } else {
            self.summary.clone()
        };
        Ok(Self {
            state: observation.state(),
            page: observation.page().clone(),
            summary,
            units: units.into_values().collect(),
            order,
        })
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
    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }

    #[must_use]
    pub fn units(&self) -> &[SemanticUnit] {
        &self.units
    }

    #[must_use]
    pub fn order(&self) -> &[ElementRef] {
        &self.order
    }
}

fn validate_exact_order(
    units: &[SemanticUnit],
    order: &[ElementRef],
) -> Result<(), OperationFailure> {
    if order.len() != units.len()
        || order.iter().copied().collect::<BTreeSet<_>>()
            != units
                .iter()
                .map(SemanticUnit::reference)
                .collect::<BTreeSet<_>>()
    {
        return Err(invalid_transition("snapshot_full_order"));
    }
    Ok(())
}

fn invalid_transition(field: &'static str) -> OperationFailure {
    OperationFailure::invalid_input(field, ValidationIssue::InvalidTransition)
}
