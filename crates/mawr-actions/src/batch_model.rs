use mawr_core::{
    AbsoluteUrl, ActionKind, BatchFailurePolicy, ElementRef, FailureClass, OperationKind, StateId,
};

use crate::{ActionExecutionFailure, ActionOutcome, SideEffectStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchAuditPhase {
    Authorized,
    PreflightRejected,
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchSkipReason {
    PriorFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchAuditEvent {
    index: usize,
    requested: ActionKind,
    effective: Option<ActionKind>,
    expected_state: StateId,
    operation: OperationKind,
    target: Option<ElementRef>,
    destination: Option<AbsoluteUrl>,
    phase: BatchAuditPhase,
    failure_class: Option<FailureClass>,
    side_effect: Option<SideEffectStatus>,
}

impl BatchAuditEvent {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn new(
        index: usize,
        requested: ActionKind,
        effective: Option<ActionKind>,
        expected_state: StateId,
        operation: OperationKind,
        target: Option<ElementRef>,
        destination: Option<AbsoluteUrl>,
        phase: BatchAuditPhase,
        failure_class: Option<FailureClass>,
        side_effect: Option<SideEffectStatus>,
    ) -> Self {
        Self {
            index,
            requested,
            effective,
            expected_state,
            operation,
            target,
            destination,
            phase,
            failure_class,
            side_effect,
        }
    }

    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }
    #[must_use]
    pub const fn requested(&self) -> ActionKind {
        self.requested
    }
    #[must_use]
    pub const fn effective(&self) -> Option<ActionKind> {
        self.effective
    }
    #[must_use]
    pub const fn expected_state(&self) -> StateId {
        self.expected_state
    }
    #[must_use]
    pub const fn operation(&self) -> OperationKind {
        self.operation
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
    pub const fn phase(&self) -> BatchAuditPhase {
        self.phase
    }
    #[must_use]
    pub const fn failure_class(&self) -> Option<FailureClass> {
        self.failure_class
    }
    #[must_use]
    pub const fn side_effect(&self) -> Option<SideEffectStatus> {
        self.side_effect
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchDiagnostics {
    action_count: usize,
    executed_count: usize,
    failure_count: usize,
    decision_boundaries_avoided: usize,
    preflight_latency_micros: u64,
    execution_latency_micros: u64,
}

impl BatchDiagnostics {
    pub(crate) const fn new(
        action_count: usize,
        executed_count: usize,
        failure_count: usize,
        preflight_latency_micros: u64,
        execution_latency_micros: u64,
    ) -> Self {
        Self {
            action_count,
            executed_count,
            failure_count,
            decision_boundaries_avoided: action_count.saturating_sub(1),
            preflight_latency_micros,
            execution_latency_micros,
        }
    }

    #[must_use]
    pub const fn action_count(self) -> usize {
        self.action_count
    }
    #[must_use]
    pub const fn executed_count(self) -> usize {
        self.executed_count
    }
    #[must_use]
    pub const fn failure_count(self) -> usize {
        self.failure_count
    }
    #[must_use]
    pub const fn decision_boundaries_avoided(self) -> usize {
        self.decision_boundaries_avoided
    }
    #[must_use]
    pub const fn preflight_latency_micros(self) -> u64 {
        self.preflight_latency_micros
    }
    #[must_use]
    pub const fn execution_latency_micros(self) -> u64 {
        self.execution_latency_micros
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchItemResult {
    Succeeded(Box<ActionOutcome>),
    Failed(ActionExecutionFailure),
    Skipped(BatchSkipReason),
}

impl BatchItemResult {
    #[must_use]
    pub const fn succeeded(&self) -> Option<&ActionOutcome> {
        match self {
            Self::Succeeded(outcome) => Some(outcome),
            _ => None,
        }
    }
    #[must_use]
    pub const fn failure(&self) -> Option<&ActionExecutionFailure> {
        match self {
            Self::Failed(failure) => Some(failure),
            _ => None,
        }
    }
    #[must_use]
    pub const fn skip_reason(&self) -> Option<BatchSkipReason> {
        match self {
            Self::Skipped(reason) => Some(*reason),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchOutcome {
    initial_state: StateId,
    final_state: StateId,
    failure_policy: BatchFailurePolicy,
    items: Vec<BatchItemResult>,
    audit_events: Vec<BatchAuditEvent>,
    diagnostics: BatchDiagnostics,
}

impl BatchOutcome {
    pub(crate) fn new(
        initial_state: StateId,
        final_state: StateId,
        failure_policy: BatchFailurePolicy,
        items: Vec<BatchItemResult>,
        audit_events: Vec<BatchAuditEvent>,
        diagnostics: BatchDiagnostics,
    ) -> Self {
        Self {
            initial_state,
            final_state,
            failure_policy,
            items,
            audit_events,
            diagnostics,
        }
    }
    #[must_use]
    pub const fn initial_state(&self) -> StateId {
        self.initial_state
    }
    #[must_use]
    pub const fn final_state(&self) -> StateId {
        self.final_state
    }
    #[must_use]
    pub const fn failure_policy(&self) -> BatchFailurePolicy {
        self.failure_policy
    }
    #[must_use]
    pub fn items(&self) -> &[BatchItemResult] {
        &self.items
    }
    #[must_use]
    pub fn audit_events(&self) -> &[BatchAuditEvent] {
        &self.audit_events
    }
    #[must_use]
    pub const fn diagnostics(&self) -> BatchDiagnostics {
        self.diagnostics
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchPreflightFailure {
    index: usize,
    failure: ActionExecutionFailure,
    audit_events: Vec<BatchAuditEvent>,
    diagnostics: BatchDiagnostics,
}

impl BatchPreflightFailure {
    pub(crate) fn new(
        index: usize,
        failure: ActionExecutionFailure,
        audit_events: Vec<BatchAuditEvent>,
        diagnostics: BatchDiagnostics,
    ) -> Self {
        Self {
            index,
            failure,
            audit_events,
            diagnostics,
        }
    }
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }
    #[must_use]
    pub const fn failure(&self) -> &ActionExecutionFailure {
        &self.failure
    }
    #[must_use]
    pub fn audit_events(&self) -> &[BatchAuditEvent] {
        &self.audit_events
    }
    #[must_use]
    pub const fn diagnostics(&self) -> BatchDiagnostics {
        self.diagnostics
    }
}
