//! Authorized deterministic actions over MAWR's native static engine.
//!
//! The executor validates state, references, native HTML semantics, capability,
//! and caller authorization before any local mutation or network request.

mod batch;
mod batch_model;
mod executor;
mod model;

pub use batch_model::{
    BatchAuditEvent, BatchAuditPhase, BatchDiagnostics, BatchItemResult, BatchOutcome,
    BatchPreflightFailure, BatchSkipReason,
};
pub use executor::StaticActionExecutor;
pub use model::{
    ActionAuthorizationContext, ActionAuthorizer, ActionDiagnostics, ActionExecutionFailure,
    ActionOutcome, AuthorizationDecision, NetworkEvidence, SideEffectStatus,
};
