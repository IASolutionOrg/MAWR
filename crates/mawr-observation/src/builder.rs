use std::num::NonZeroU64;
use std::time::Instant;

use mawr_core::{
    CapabilityReport, CollectionLimit, EngineFailureKind, Measurement, MeasurementKind,
    MeasurementSet, MeasurementSource, Observation, ObservationBasis, ObservationRequest,
    OperationFailure, Property, Relationship, ResetReason, ResourceKind, SemanticUnit,
    SemanticValue, StateId, UnavailableReason,
};
use mawr_state::{SemanticStateStore, StoredSemanticUnit, StoredState};

const DEFAULT_UNIT_LIMIT: u64 = 250_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FullObservationConfig {
    unit_limit: CollectionLimit,
}

impl FullObservationConfig {
    #[must_use]
    pub const fn with_unit_limit(mut self, unit_limit: CollectionLimit) -> Self {
        self.unit_limit = unit_limit;
        self
    }

    #[must_use]
    pub const fn unit_limit(self) -> CollectionLimit {
        self.unit_limit
    }
}

impl Default for FullObservationConfig {
    fn default() -> Self {
        Self {
            unit_limit: CollectionLimit::new(DEFAULT_UNIT_LIMIT, "observation_unit_limit")
                .expect("default observation unit limit is valid"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationBuildDiagnostics {
    construction_latency_micros: u64,
    unit_count: usize,
    relationship_count: usize,
    unresolved_relationship_count: usize,
    logical_content_bytes: u64,
    source_input_bytes: u64,
    goal_deferred: bool,
    token_budget_deferred: bool,
}

impl ObservationBuildDiagnostics {
    #[must_use]
    pub const fn construction_latency_micros(&self) -> u64 {
        self.construction_latency_micros
    }

    #[must_use]
    pub const fn unit_count(&self) -> usize {
        self.unit_count
    }

    #[must_use]
    pub const fn relationship_count(&self) -> usize {
        self.relationship_count
    }

    #[must_use]
    pub const fn unresolved_relationship_count(&self) -> usize {
        self.unresolved_relationship_count
    }

    #[must_use]
    pub const fn logical_content_bytes(&self) -> u64 {
        self.logical_content_bytes
    }

    #[must_use]
    pub const fn source_input_bytes(&self) -> u64 {
        self.source_input_bytes
    }

    #[must_use]
    pub const fn goal_deferred(&self) -> bool {
        self.goal_deferred
    }

    #[must_use]
    pub const fn token_budget_deferred(&self) -> bool {
        self.token_budget_deferred
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltObservation {
    observation: Observation,
    diagnostics: ObservationBuildDiagnostics,
}

impl BuiltObservation {
    #[must_use]
    pub const fn observation(&self) -> &Observation {
        &self.observation
    }

    #[must_use]
    pub const fn diagnostics(&self) -> &ObservationBuildDiagnostics {
        &self.diagnostics
    }

    #[must_use]
    pub fn into_observation(self) -> Observation {
        self.observation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullObservationBuilder {
    capabilities: CapabilityReport,
    config: FullObservationConfig,
}

impl FullObservationBuilder {
    #[must_use]
    pub const fn new(capabilities: CapabilityReport, config: FullObservationConfig) -> Self {
        Self {
            capabilities,
            config,
        }
    }

    pub fn build(
        &self,
        store: &SemanticStateStore,
        request: &ObservationRequest,
    ) -> Result<BuiltObservation, OperationFailure> {
        if request.session() != store.session() {
            return Err(OperationFailure::session_mismatch(
                "observation_request_session",
                store.session(),
                request.session(),
            ));
        }
        let current = store
            .current()
            .ok_or_else(|| OperationFailure::EngineFailure {
                engine: store.engine().clone(),
                kind: EngineFailureKind::StateUnavailable,
            })?;
        if current.units().len() > self.config.unit_limit().get() as usize {
            return Err(OperationFailure::ResourceLimit {
                resource: ResourceKind::SemanticUnits,
                configured_limit: NonZeroU64::new(self.config.unit_limit().get())
                    .expect("collection limit is non-zero"),
            });
        }

        let started = Instant::now();
        let basis = observation_basis(store, current, request.requested_base())?;
        let summary = page_summary(current);
        let units = current
            .units()
            .iter()
            .map(convert_unit)
            .collect::<Result<Vec<_>, _>>()?;
        let relationship_count = units.iter().map(|unit| unit.relationships().count()).sum();
        let unresolved_relationship_count = current
            .units()
            .iter()
            .map(|unit| unit.unresolved_relationships().len())
            .sum();
        let logical_content_bytes = logical_content_bytes(current, &summary);

        let observation = Observation::new(
            current.id(),
            current.page().clone(),
            store.engine().clone(),
            self.capabilities.clone(),
            basis,
            self.config.unit_limit(),
        )
        .map_err(OperationFailure::InvalidInput)?
        .with_summary(summary)
        .map_err(OperationFailure::InvalidInput)?
        .with_units(units)
        .map_err(OperationFailure::InvalidInput)?;
        let construction_latency_micros =
            u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        let measurements = MeasurementSet::unavailable_all(UnavailableReason::NotMeasured)
            .with(
                MeasurementKind::LatencyMicros,
                Measurement::Exact {
                    value: construction_latency_micros,
                    source: MeasurementSource::RuntimeCounter,
                },
            )
            .with(
                MeasurementKind::ObservationTokens,
                Measurement::Unavailable(UnavailableReason::SourceMissing),
            )
            .with(
                MeasurementKind::CpuMicros,
                Measurement::Unavailable(UnavailableReason::SourceMissing),
            )
            .with(
                MeasurementKind::PeakMemoryBytes,
                Measurement::Unavailable(UnavailableReason::SourceMissing),
            );
        let observation = observation.with_measurements(measurements);
        let diagnostics = ObservationBuildDiagnostics {
            construction_latency_micros,
            unit_count: observation.units().len(),
            relationship_count,
            unresolved_relationship_count,
            logical_content_bytes,
            source_input_bytes: current.document().diagnostics().input_bytes(),
            goal_deferred: request.goal().is_some(),
            token_budget_deferred: request.max_tokens().is_some(),
        };
        Ok(BuiltObservation {
            observation,
            diagnostics,
        })
    }
}

fn observation_basis(
    store: &SemanticStateStore,
    current: &StoredState,
    requested_base: Option<StateId>,
) -> Result<ObservationBasis, OperationFailure> {
    let Some(base) = requested_base else {
        return Ok(ObservationBasis::Full(if current.id().sequence() == 1 {
            mawr_core::FullObservationReason::Initial
        } else {
            mawr_core::FullObservationReason::NoBaseRequested
        }));
    };
    match store.state(base) {
        Ok(base_state) if base_state.page().id() == current.page().id() => {
            Ok(ObservationBasis::Incremental { base })
        }
        Ok(_) => Ok(ObservationBasis::Reset {
            requested_base: base,
            reason: ResetReason::NavigationBoundary,
        }),
        Err(OperationFailure::StaleState { .. }) => Ok(ObservationBasis::Reset {
            requested_base: base,
            reason: if base.sequence() < current.id().sequence() {
                ResetReason::BaseEvicted
            } else {
                ResetReason::BaseUnavailable
            },
        }),
        Err(failure) => Err(failure),
    }
}

fn page_summary(state: &StoredState) -> String {
    state
        .document()
        .title()
        .unwrap_or("Untitled page")
        .to_owned()
}

fn convert_unit(stored: &StoredSemanticUnit) -> Result<SemanticUnit, OperationFailure> {
    let extracted = stored.semantic();
    let mut unit = SemanticUnit::new(stored.reference(), extracted.role(), extracted.provenance())
        .with_name_property(extracted.name().clone())
        .with_description_property(extracted.description().clone())
        .with_value(extracted.value().clone())
        .with_state(extracted.state().clone())
        .with_destination(extracted.destination().clone());
    if let Some(parent) = stored.parent() {
        unit = unit
            .with_parent(parent)
            .map_err(OperationFailure::InvalidInput)?;
    }
    for relationship in stored.relationships() {
        unit = unit
            .with_relationship(Relationship::new(
                relationship.kind(),
                relationship.target(),
            ))
            .map_err(OperationFailure::InvalidInput)?;
    }
    for affordance in extracted.affordances().iter() {
        unit = unit.with_affordance(affordance);
    }
    Ok(unit)
}

fn logical_content_bytes(state: &StoredState, summary: &str) -> u64 {
    let mut bytes =
        saturating_len(summary).saturating_add(saturating_len(state.page().url().as_str()));
    for unit in state.units() {
        let semantic = unit.semantic();
        bytes = bytes
            .saturating_add(property_text_bytes(semantic.name()))
            .saturating_add(property_text_bytes(semantic.description()))
            .saturating_add(match semantic.value() {
                SemanticValue::Absent | SemanticValue::Redacted => 0,
                SemanticValue::Text(value) => saturating_len(value.as_str()),
                SemanticValue::Unknown(reason) => saturating_len(reason.as_str()),
            })
            .saturating_add(match semantic.destination() {
                Property::Known(destination) => saturating_len(destination.as_str()),
                Property::NotApplicable | Property::Unknown(_) => 0,
            });
    }
    bytes
}

fn property_text_bytes<const MAX: usize>(property: &Property<mawr_core::BoundedText<MAX>>) -> u64 {
    match property {
        Property::Known(value) => saturating_len(value.as_str()),
        Property::NotApplicable | Property::Unknown(_) => 0,
    }
}

fn saturating_len(value: &str) -> u64 {
    u64::try_from(value.len()).unwrap_or(u64::MAX)
}
