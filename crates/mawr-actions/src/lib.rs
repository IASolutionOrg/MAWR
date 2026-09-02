//! Authorized deterministic actions over MAWR's native static engine.
//!
//! The executor validates state, references, native HTML semantics, capability,
//! and caller authorization before any local mutation or network request.

mod executor;
mod model;

pub use executor::StaticActionExecutor;
pub use model::{
    ActionAuthorizationContext, ActionAuthorizer, ActionDiagnostics, ActionExecutionFailure,
    ActionOutcome, AuthorizationDecision, NetworkEvidence, SideEffectStatus,
};
