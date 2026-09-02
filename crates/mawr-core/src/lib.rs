//! Transport- and engine-independent domain contracts for MAWR.
//!
//! This crate intentionally contains no JSON, HTTP client, parser, CLI, MCP,
//! or engine-adapter dependency. Its API remains internal and evolving until
//! later milestones define versioned external encodings.

mod action;
mod address;
mod capability;
mod failure;
mod identity;
mod ids;
mod limits;
mod measurement;
mod observation;
mod semantic;
mod text;
mod transition;
mod validation;

pub use action::{
    Action, ActionBatch, ActionKind, ActionRequest, BatchFailurePolicy, MAX_ACTIONS_PER_BATCH,
    PressCommand,
};
pub use address::{AbsoluteUrl, MAX_URL_BYTES};
pub use capability::{
    Capability, CapabilityConstraint, CapabilityConstraints, CapabilityReport, CapabilityStatus,
    UnsupportedReason,
};
pub use failure::{
    AdapterFailureKind, AuthorizationReason, EngineFailureKind, FailureClass,
    NavigationFailureKind, OperationFailure, OperationKind, ParsingFailureKind,
    ProtocolFailureKind, ResourceKind, RetryDisposition,
};
pub use identity::{EngineIdentity, EngineKind, PageIdentity};
pub use ids::{ElementRef, PageId, SessionId, StateId};
pub use limits::{BoundedU64, CollectionLimit, ObservationTokenBudget};
pub use measurement::{
    Measurement, MeasurementKind, MeasurementSet, MeasurementSource, UnavailableReason,
};
pub use observation::{
    FullObservationReason, Observation, ObservationBasis, ObservationChanges, ObservationRequest,
    OmissionCategory, OmissionSummary,
};
pub use semantic::{
    ActionAffordances, ElementState, MAX_SEMANTIC_DESCRIPTION_BYTES, MAX_SEMANTIC_NAME_BYTES,
    Property, PropertyUnknownReason, Provenance, Relationship, RelationshipKind, SemanticRole,
    SemanticUnit, SemanticValue,
};
pub use text::{BoundedText, NonEmptyText, SensitiveText};
pub use transition::{ResetReason, StateTransition, TransitionCause};
pub use validation::{ValidationError, ValidationIssue};
