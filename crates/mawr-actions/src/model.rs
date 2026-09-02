use mawr_core::{
    AbsoluteUrl, ActionKind, AuthorizationReason, ElementRef, OperationFailure, OperationKind,
    StateId,
};
use mawr_native_static::RequestMethod;
use mawr_state::StateUpdate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationDecision {
    Allow,
    Deny(AuthorizationReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionAuthorizationContext {
    operation: OperationKind,
    requested: ActionKind,
    effective: ActionKind,
    expected_state: StateId,
    target: Option<ElementRef>,
    destination: Option<AbsoluteUrl>,
    method: Option<RequestMethod>,
}

impl ActionAuthorizationContext {
    pub(crate) const fn new(
        operation: OperationKind,
        requested: ActionKind,
        effective: ActionKind,
        expected_state: StateId,
        target: Option<ElementRef>,
        destination: Option<AbsoluteUrl>,
        method: Option<RequestMethod>,
    ) -> Self {
        Self {
            operation,
            requested,
            effective,
            expected_state,
            target,
            destination,
            method,
        }
    }

    #[must_use]
    pub const fn operation(&self) -> OperationKind {
        self.operation
    }
    #[must_use]
    pub const fn requested(&self) -> ActionKind {
        self.requested
    }
    #[must_use]
    pub const fn effective(&self) -> ActionKind {
        self.effective
    }
    #[must_use]
    pub const fn expected_state(&self) -> StateId {
        self.expected_state
    }
    #[must_use]
    pub const fn target(&self) -> Option<ElementRef> {
        self.target
    }
    #[must_use]
    pub const fn destination(&self) -> Option<&AbsoluteUrl> {
        self.destination.as_ref()
    }
    #[must_use]
    pub const fn method(&self) -> Option<RequestMethod> {
        self.method
    }
}

pub trait ActionAuthorizer {
    fn authorize(&self, context: &ActionAuthorizationContext) -> AuthorizationDecision;
}

impl<F> ActionAuthorizer for F
where
    F: Fn(&ActionAuthorizationContext) -> AuthorizationDecision,
{
    fn authorize(&self, context: &ActionAuthorizationContext) -> AuthorizationDecision {
        self(context)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkEvidence {
    method: RequestMethod,
    requested_url: AbsoluteUrl,
    final_url: AbsoluteUrl,
    status: u16,
    request_count: u32,
    redirect_count: u32,
    decoded_body_bytes: u64,
}

impl NetworkEvidence {
    pub(crate) fn from_document(
        method: RequestMethod,
        document: &mawr_native_static::DocumentInput,
    ) -> Self {
        Self {
            method,
            requested_url: document.requested_url().clone(),
            final_url: document.final_url().clone(),
            status: document.status(),
            request_count: document.diagnostics().request_count(),
            redirect_count: document.diagnostics().redirect_count(),
            decoded_body_bytes: document.diagnostics().decoded_body_bytes(),
        }
    }

    #[must_use]
    pub const fn method(&self) -> RequestMethod {
        self.method
    }
    #[must_use]
    pub const fn requested_url(&self) -> &AbsoluteUrl {
        &self.requested_url
    }
    #[must_use]
    pub const fn final_url(&self) -> &AbsoluteUrl {
        &self.final_url
    }
    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }
    #[must_use]
    pub const fn request_count(&self) -> u32 {
        self.request_count
    }
    #[must_use]
    pub const fn redirect_count(&self) -> u32 {
        self.redirect_count
    }
    #[must_use]
    pub const fn decoded_body_bytes(&self) -> u64 {
        self.decoded_body_bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideEffectStatus {
    NotStarted,
    Requested,
    NetworkCompleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionDiagnostics {
    latency_micros: u64,
}

impl ActionDiagnostics {
    pub(crate) const fn new(latency_micros: u64) -> Self {
        Self { latency_micros }
    }
    #[must_use]
    pub const fn latency_micros(self) -> u64 {
        self.latency_micros
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutcome {
    requested: ActionKind,
    effective: ActionKind,
    update: StateUpdate,
    network: Option<NetworkEvidence>,
    diagnostics: ActionDiagnostics,
}

impl ActionOutcome {
    pub(crate) const fn new(
        requested: ActionKind,
        effective: ActionKind,
        update: StateUpdate,
        network: Option<NetworkEvidence>,
        diagnostics: ActionDiagnostics,
    ) -> Self {
        Self {
            requested,
            effective,
            update,
            network,
            diagnostics,
        }
    }
    #[must_use]
    pub const fn requested(&self) -> ActionKind {
        self.requested
    }
    #[must_use]
    pub const fn effective(&self) -> ActionKind {
        self.effective
    }
    #[must_use]
    pub const fn update(&self) -> &StateUpdate {
        &self.update
    }
    #[must_use]
    pub const fn network(&self) -> Option<&NetworkEvidence> {
        self.network.as_ref()
    }
    #[must_use]
    pub const fn diagnostics(&self) -> ActionDiagnostics {
        self.diagnostics
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionExecutionFailure {
    failure: Box<OperationFailure>,
    side_effect: SideEffectStatus,
    network: Option<Box<NetworkEvidence>>,
}

impl ActionExecutionFailure {
    pub(crate) fn new(
        failure: OperationFailure,
        side_effect: SideEffectStatus,
        network: Option<NetworkEvidence>,
    ) -> Self {
        Self {
            failure: Box::new(failure),
            side_effect,
            network: network.map(Box::new),
        }
    }
    pub(crate) fn preflight(failure: OperationFailure) -> Self {
        Self::new(failure, SideEffectStatus::NotStarted, None)
    }
    #[must_use]
    pub const fn failure(&self) -> &OperationFailure {
        &self.failure
    }
    #[must_use]
    pub const fn side_effect(&self) -> SideEffectStatus {
        self.side_effect
    }
    #[must_use]
    pub fn network(&self) -> Option<&NetworkEvidence> {
        self.network.as_deref()
    }
}
