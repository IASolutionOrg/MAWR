use std::collections::{BTreeMap, VecDeque};
use std::num::NonZeroU64;
use std::time::Instant;

use mawr_core::{
    ElementRef, EngineIdentity, OperationFailure, PageId, PageIdentity, ResetReason, ResourceKind,
    SessionId, StateId, StateTransition, TransitionCause,
};
use mawr_semantic_html::{ExtractedRelationship, SemanticDocument, SourceNodeId};

use crate::config::StateStoreConfig;
use crate::matcher;
use crate::model::{
    StableRelationship, StateDiagnostics, StateUpdate, StoredSemanticUnit, StoredState,
};

#[derive(Debug)]
pub struct SemanticStateStore {
    session: SessionId,
    engine: EngineIdentity,
    config: StateStoreConfig,
    states: VecDeque<StoredState>,
    retained_units: usize,
    next_state_sequence: u64,
    next_page_sequence: u64,
    next_element_sequence: u64,
}

impl SemanticStateStore {
    #[must_use]
    pub fn new(session: SessionId, engine: EngineIdentity, config: StateStoreConfig) -> Self {
        Self {
            session,
            engine,
            config,
            states: VecDeque::new(),
            retained_units: 0,
            next_state_sequence: 1,
            next_page_sequence: 1,
            next_element_sequence: 1,
        }
    }

    #[must_use]
    pub const fn session(&self) -> SessionId {
        self.session
    }

    #[must_use]
    pub const fn config(&self) -> StateStoreConfig {
        self.config
    }

    #[must_use]
    pub fn current(&self) -> Option<&StoredState> {
        self.states.back()
    }

    pub fn update(
        &mut self,
        document: SemanticDocument,
        requested_cause: TransitionCause,
    ) -> Result<StateUpdate, OperationFailure> {
        if document.session() != self.session {
            return Err(OperationFailure::session_mismatch(
                "semantic_document_session",
                self.session,
                document.session(),
            ));
        }
        if document.units().len() > self.config.retained_units() {
            return Err(OperationFailure::ResourceLimit {
                resource: ResourceKind::StateRetention,
                configured_limit: NonZeroU64::new(self.config.retained_units() as u64)
                    .expect("configured retention is non-zero"),
            });
        }

        let previous = self.current();
        let reset = reset_reason(previous, &document, requested_cause);
        let effective_cause = effective_cause(previous, requested_cause, reset);
        let retained_page = previous
            .filter(|_| reset.is_none())
            .map(|state| state.page().clone());
        let previous_state_id = previous.map(StoredState::id);
        let mut next_element_sequence = self.next_element_sequence;
        let mut next_page_sequence = self.next_page_sequence;
        let mut next_state_sequence = self.next_state_sequence;
        let started = Instant::now();
        let matched = matcher::assign(
            self.session,
            next_element_sequence,
            previous,
            document.units(),
            reset,
        )?;
        let matching_latency_micros =
            u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        next_element_sequence = matched.next_sequence;
        let page = if let Some(page) = retained_page {
            page
        } else {
            let id = next_page_id(self.session, &mut next_page_sequence)?;
            PageIdentity::new(id, document.document_url().clone())
        };
        let state_id = next_state_id(self.session, &mut next_state_sequence)?;

        let state = build_state(state_id, page.clone(), document, matched.references);
        let transition = StateTransition::new(
            previous_state_id,
            state_id,
            page,
            self.engine.clone(),
            effective_cause,
        )
        .map_err(OperationFailure::InvalidInput)?;

        self.next_element_sequence = next_element_sequence;
        self.next_page_sequence = next_page_sequence;
        self.next_state_sequence = next_state_sequence;
        self.retained_units = self.retained_units.saturating_add(state.units.len());
        self.states.push_back(state);
        let evicted_states = self.evict_to_limits();
        let diagnostics = StateDiagnostics {
            reset,
            matching_latency_micros,
            preserved_references: matched.preserved,
            new_references: matched.assignments.len() - matched.preserved,
            assignments: matched.assignments,
            losses: matched.losses,
            evicted_states,
            retained_states: self.states.len(),
            retained_units: self.retained_units,
        };
        Ok(StateUpdate {
            transition,
            diagnostics,
        })
    }

    pub fn state(&self, id: StateId) -> Result<&StoredState, OperationFailure> {
        if id.session() != self.session {
            return Err(OperationFailure::session_mismatch(
                "state_session",
                self.session,
                id.session(),
            ));
        }
        self.states
            .iter()
            .find(|state| state.id() == id)
            .ok_or_else(|| OperationFailure::StaleState {
                expected: id,
                actual: self.current().map(StoredState::id),
            })
    }

    pub fn resolve_current(
        &self,
        expected_state: StateId,
        reference: ElementRef,
    ) -> Result<&StoredSemanticUnit, OperationFailure> {
        if expected_state.session() != self.session {
            return Err(OperationFailure::session_mismatch(
                "expected_state_session",
                self.session,
                expected_state.session(),
            ));
        }
        if reference.session() != self.session {
            return Err(OperationFailure::session_mismatch(
                "element_reference_session",
                self.session,
                reference.session(),
            ));
        }
        let Some(current) = self.current() else {
            return Err(OperationFailure::StaleState {
                expected: expected_state,
                actual: None,
            });
        };
        if current.id() != expected_state {
            return Err(OperationFailure::StaleState {
                expected: expected_state,
                actual: Some(current.id()),
            });
        }
        current
            .unit(reference)
            .ok_or(OperationFailure::MissingReference { reference })
    }

    pub fn retained_state_ids(&self) -> impl Iterator<Item = StateId> + '_ {
        self.states.iter().map(StoredState::id)
    }

    fn evict_to_limits(&mut self) -> Vec<StateId> {
        let mut evicted = Vec::new();
        while self.states.len() > self.config.retained_states()
            || self.retained_units > self.config.retained_units()
        {
            let state = self
                .states
                .pop_front()
                .expect("limits retain at least the current state");
            self.retained_units -= state.units.len();
            evicted.push(state.id());
        }
        evicted
    }
}

fn reset_reason(
    previous: Option<&StoredState>,
    document: &SemanticDocument,
    cause: TransitionCause,
) -> Option<ResetReason> {
    let previous = previous?;
    match cause {
        TransitionCause::Navigation => Some(ResetReason::NavigationBoundary),
        TransitionCause::Reset(reason) => Some(reason),
        _ if previous.page().url() != document.document_url() => {
            Some(ResetReason::NavigationBoundary)
        }
        _ => None,
    }
}

fn effective_cause(
    previous: Option<&StoredState>,
    requested: TransitionCause,
    reset: Option<ResetReason>,
) -> TransitionCause {
    match (previous, reset, requested) {
        (None, _, TransitionCause::Navigation) => TransitionCause::Navigation,
        (None, _, _) => TransitionCause::Initial,
        (Some(_), Some(ResetReason::NavigationBoundary), _) => TransitionCause::Navigation,
        (Some(_), Some(reason), _) => TransitionCause::Reset(reason),
        (Some(_), None, cause) => cause,
    }
}

fn build_state(
    id: StateId,
    page: PageIdentity,
    document: SemanticDocument,
    references: Vec<ElementRef>,
) -> StoredState {
    let source_to_reference = document
        .units()
        .iter()
        .zip(&references)
        .map(|(unit, reference)| (unit.source(), *reference))
        .collect::<BTreeMap<SourceNodeId, ElementRef>>();
    let units = document
        .units()
        .iter()
        .cloned()
        .zip(references)
        .map(|(semantic, reference)| {
            let parent = semantic
                .parent_source()
                .and_then(|source| source_to_reference.get(&source).copied());
            let (relationships, unresolved_relationships) =
                stable_relationships(semantic.relationships(), &source_to_reference);
            StoredSemanticUnit {
                reference,
                parent,
                semantic,
                relationships,
                unresolved_relationships,
            }
        })
        .collect::<Vec<_>>();
    let reference_index = units
        .iter()
        .enumerate()
        .map(|(index, unit)| (unit.reference(), index))
        .collect();
    StoredState {
        id,
        page,
        document,
        units,
        reference_index,
    }
}

fn stable_relationships(
    relationships: &[ExtractedRelationship],
    references: &BTreeMap<SourceNodeId, ElementRef>,
) -> (Vec<StableRelationship>, Vec<ExtractedRelationship>) {
    let mut stable = Vec::new();
    let mut unresolved = Vec::new();
    for relationship in relationships {
        if let Some(target) = references.get(&relationship.target()) {
            stable.push(StableRelationship::new(relationship.kind(), *target));
        } else {
            unresolved.push(*relationship);
        }
    }
    (stable, unresolved)
}

fn next_state_id(session: SessionId, sequence: &mut u64) -> Result<StateId, OperationFailure> {
    let id = StateId::new(session, *sequence).map_err(OperationFailure::InvalidInput)?;
    *sequence = sequence.checked_add(1).ok_or_else(sequence_exhausted)?;
    Ok(id)
}

fn next_page_id(session: SessionId, sequence: &mut u64) -> Result<PageId, OperationFailure> {
    let id = PageId::new(session, *sequence).map_err(OperationFailure::InvalidInput)?;
    *sequence = sequence.checked_add(1).ok_or_else(sequence_exhausted)?;
    Ok(id)
}

fn sequence_exhausted() -> OperationFailure {
    OperationFailure::ResourceLimit {
        resource: ResourceKind::StateRetention,
        configured_limit: NonZeroU64::new(u64::MAX).expect("u64::MAX is non-zero"),
    }
}
