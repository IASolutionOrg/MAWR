use std::collections::BTreeMap;

use mawr_core::{
    ElementRef, PageIdentity, RelationshipKind, ResetReason, StateId, StateTransition,
};
use mawr_semantic_html::{
    ExtractedRelationship, ExtractedSemanticUnit, SemanticDocument, SourceNodeId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableRelationship {
    kind: RelationshipKind,
    target: ElementRef,
}

impl StableRelationship {
    #[must_use]
    pub const fn new(kind: RelationshipKind, target: ElementRef) -> Self {
        Self { kind, target }
    }

    #[must_use]
    pub const fn kind(self) -> RelationshipKind {
        self.kind
    }

    #[must_use]
    pub const fn target(self) -> ElementRef {
        self.target
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSemanticUnit {
    pub(crate) reference: ElementRef,
    pub(crate) parent: Option<ElementRef>,
    pub(crate) semantic: ExtractedSemanticUnit,
    pub(crate) relationships: Vec<StableRelationship>,
    pub(crate) unresolved_relationships: Vec<ExtractedRelationship>,
}

impl StoredSemanticUnit {
    #[must_use]
    pub const fn reference(&self) -> ElementRef {
        self.reference
    }

    #[must_use]
    pub const fn parent(&self) -> Option<ElementRef> {
        self.parent
    }

    #[must_use]
    pub const fn semantic(&self) -> &ExtractedSemanticUnit {
        &self.semantic
    }

    #[must_use]
    pub fn relationships(&self) -> &[StableRelationship] {
        &self.relationships
    }

    #[must_use]
    pub fn unresolved_relationships(&self) -> &[ExtractedRelationship] {
        &self.unresolved_relationships
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct StoredState {
    pub(crate) id: StateId,
    pub(crate) page: PageIdentity,
    pub(crate) document: SemanticDocument,
    pub(crate) units: Vec<StoredSemanticUnit>,
    pub(crate) reference_index: BTreeMap<ElementRef, usize>,
}

impl StoredState {
    #[must_use]
    pub const fn id(&self) -> StateId {
        self.id
    }

    #[must_use]
    pub const fn page(&self) -> &PageIdentity {
        &self.page
    }

    #[must_use]
    pub const fn document(&self) -> &SemanticDocument {
        &self.document
    }

    #[must_use]
    pub fn units(&self) -> &[StoredSemanticUnit] {
        &self.units
    }

    #[must_use]
    pub fn unit(&self, reference: ElementRef) -> Option<&StoredSemanticUnit> {
        self.reference_index
            .get(&reference)
            .map(|index| &self.units[*index])
    }
}

impl std::fmt::Debug for StoredState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StoredState")
            .field("id", &self.id)
            .field("page", &self.page)
            .field("units", &self.units)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceAssignmentReason {
    InitialState,
    Reset(ResetReason),
    UniqueAuthorId,
    UniqueSemanticIdentity,
    NewElement,
    NoStableIdentity,
    AmbiguousAuthorId,
    AmbiguousSemanticIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceAssignment {
    source: SourceNodeId,
    reference: ElementRef,
    reason: ReferenceAssignmentReason,
}

impl ReferenceAssignment {
    pub(crate) const fn new(
        source: SourceNodeId,
        reference: ElementRef,
        reason: ReferenceAssignmentReason,
    ) -> Self {
        Self {
            source,
            reference,
            reason,
        }
    }

    #[must_use]
    pub const fn source(self) -> SourceNodeId {
        self.source
    }

    #[must_use]
    pub const fn reference(self) -> ElementRef {
        self.reference
    }

    #[must_use]
    pub const fn reason(self) -> ReferenceAssignmentReason {
        self.reason
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceLossReason {
    Reset(ResetReason),
    RemovedOrIdentityChanged,
    AmbiguousAuthorId,
    AmbiguousSemanticIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceLoss {
    reference: ElementRef,
    reason: ReferenceLossReason,
}

impl ReferenceLoss {
    pub(crate) const fn new(reference: ElementRef, reason: ReferenceLossReason) -> Self {
        Self { reference, reason }
    }

    #[must_use]
    pub const fn reference(self) -> ElementRef {
        self.reference
    }

    #[must_use]
    pub const fn reason(self) -> ReferenceLossReason {
        self.reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDiagnostics {
    pub(crate) reset: Option<ResetReason>,
    pub(crate) matching_latency_micros: u64,
    pub(crate) preserved_references: usize,
    pub(crate) new_references: usize,
    pub(crate) assignments: Vec<ReferenceAssignment>,
    pub(crate) losses: Vec<ReferenceLoss>,
    pub(crate) evicted_states: Vec<StateId>,
    pub(crate) retained_states: usize,
    pub(crate) retained_units: usize,
}

impl StateDiagnostics {
    #[must_use]
    pub const fn reset(&self) -> Option<ResetReason> {
        self.reset
    }

    #[must_use]
    pub const fn matching_latency_micros(&self) -> u64 {
        self.matching_latency_micros
    }

    #[must_use]
    pub const fn preserved_references(&self) -> usize {
        self.preserved_references
    }

    #[must_use]
    pub const fn new_references(&self) -> usize {
        self.new_references
    }

    #[must_use]
    pub fn assignments(&self) -> &[ReferenceAssignment] {
        &self.assignments
    }

    #[must_use]
    pub fn losses(&self) -> &[ReferenceLoss] {
        &self.losses
    }

    #[must_use]
    pub fn evicted_states(&self) -> &[StateId] {
        &self.evicted_states
    }

    #[must_use]
    pub const fn retained_states(&self) -> usize {
        self.retained_states
    }

    #[must_use]
    pub const fn retained_units(&self) -> usize {
        self.retained_units
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateUpdate {
    pub(crate) transition: StateTransition,
    pub(crate) diagnostics: StateDiagnostics,
}

impl StateUpdate {
    #[must_use]
    pub const fn transition(&self) -> &StateTransition {
        &self.transition
    }

    #[must_use]
    pub const fn diagnostics(&self) -> &StateDiagnostics {
        &self.diagnostics
    }
}
