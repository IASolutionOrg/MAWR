use std::time::{Duration, Instant};

use mawr_core::{
    Action, ActionBatch, ActionKind, BatchFailurePolicy, OperationFailure, OperationKind, StateId,
    ValidationIssue,
};
use mawr_native_static::CancellationToken;

use crate::batch_model::{
    BatchAuditEvent, BatchAuditPhase, BatchDiagnostics, BatchItemResult, BatchOutcome,
    BatchPreflightFailure, BatchSkipReason,
};
use crate::executor::Prepared;
use crate::{ActionAuthorizer, ActionExecutionFailure, SideEffectStatus, StaticActionExecutor};

struct PreflightItem {
    requested: ActionKind,
    expected_state: StateId,
    prepared: Prepared,
}

struct PreflightAbort {
    index: usize,
    failure: ActionExecutionFailure,
    audit_events: Vec<BatchAuditEvent>,
}

impl<A: ActionAuthorizer> StaticActionExecutor<A> {
    /// Preflights and executes a bounded ordered batch.
    ///
    /// A preflight error leaves the real semantic state untouched and performs
    /// no network request. Runtime failures are returned inside [`BatchOutcome`]
    /// so callers can observe the exact committed prefix and side-effect status.
    pub async fn execute_batch(
        &mut self,
        batch: ActionBatch,
        cancellation: &CancellationToken,
    ) -> Result<BatchOutcome, BatchPreflightFailure> {
        let initial_state = batch.expected_state();
        let failure_policy = batch.failure_policy();
        let action_count = batch.actions().len();
        let real_store = self.store.clone();
        let preflight_started = Instant::now();
        let preflight = self.preflight_batch(&batch);
        let preflight_latency = micros(preflight_started.elapsed());
        self.store = real_store;

        let (items, mut audit_events) = match preflight {
            Ok(result) => result,
            Err(abort) => {
                return Err(BatchPreflightFailure::new(
                    abort.index,
                    abort.failure,
                    abort.audit_events,
                    BatchDiagnostics::new(action_count, 0, 1, preflight_latency, 0),
                ));
            }
        };

        let execution_started = Instant::now();
        let mut results = Vec::with_capacity(action_count);
        let mut executed_count = 0;
        let mut failure_count = 0;
        let mut prior_failure = false;

        for (index, item) in items.into_iter().enumerate() {
            let can_continue_independently = failure_policy
                == BatchFailurePolicy::ContinueIndependent
                && item.requested == ActionKind::Navigate;
            if prior_failure && !can_continue_independently {
                audit_events.push(audit_from_prepared(
                    index,
                    &item.prepared,
                    BatchAuditPhase::Skipped,
                    None,
                    Some(SideEffectStatus::NotStarted),
                ));
                results.push(BatchItemResult::Skipped(BatchSkipReason::PriorFailure));
                continue;
            }

            executed_count += 1;
            let audit_prepared = item.prepared.clone();
            match self
                .execute_prepared(
                    item.requested,
                    item.expected_state,
                    item.prepared,
                    cancellation,
                    Instant::now(),
                )
                .await
            {
                Ok(outcome) => {
                    audit_events.push(audit_from_prepared(
                        index,
                        &audit_prepared,
                        BatchAuditPhase::Succeeded,
                        None,
                        None,
                    ));
                    results.push(BatchItemResult::Succeeded(Box::new(outcome)));
                }
                Err(failure) => {
                    failure_count += 1;
                    prior_failure = true;
                    audit_events.push(audit_from_prepared(
                        index,
                        &audit_prepared,
                        BatchAuditPhase::Failed,
                        Some(failure.failure().class()),
                        Some(failure.side_effect()),
                    ));
                    results.push(BatchItemResult::Failed(failure));
                }
            }
        }

        let final_state = self
            .store
            .current()
            .expect("batch preflight verified a current state")
            .id();
        Ok(BatchOutcome::new(
            initial_state,
            final_state,
            failure_policy,
            results,
            audit_events,
            BatchDiagnostics::new(
                action_count,
                executed_count,
                failure_count,
                preflight_latency,
                micros(execution_started.elapsed()),
            ),
        ))
    }

    fn preflight_batch(
        &mut self,
        batch: &ActionBatch,
    ) -> Result<(Vec<PreflightItem>, Vec<BatchAuditEvent>), PreflightAbort> {
        let mut audit_events = Vec::with_capacity(batch.actions().len() * 2);
        if let Err(failure) = self.ensure_current(batch.expected_state()) {
            let action = &batch.actions()[0];
            let requested = action.kind();
            let (target, destination) = unprepared_context(action);
            let execution_failure = ActionExecutionFailure::preflight(failure);
            audit_events.push(audit_without_prepared(
                0,
                requested,
                batch.expected_state(),
                target,
                destination,
                execution_failure.failure().class(),
            ));
            return Err(PreflightAbort {
                index: 0,
                failure: execution_failure,
                audit_events,
            });
        }

        let mut prepared_items = Vec::with_capacity(batch.actions().len());
        let mut crossed_network_boundary = false;
        for (index, action) in batch.actions().iter().cloned().enumerate() {
            let requested = action.kind();
            let (audit_target, audit_destination) = unprepared_context(&action);
            let expected_state = self
                .store
                .current()
                .expect("initial batch state was verified")
                .id();

            if crossed_network_boundary && !matches!(action, Action::Navigate(_)) {
                let execution_failure =
                    ActionExecutionFailure::preflight(OperationFailure::invalid_input(
                        "batch_navigation_boundary",
                        ValidationIssue::InvalidTransition,
                    ));
                audit_events.push(audit_without_prepared(
                    index,
                    requested,
                    expected_state,
                    audit_target,
                    audit_destination,
                    execution_failure.failure().class(),
                ));
                return Err(PreflightAbort {
                    index,
                    failure: execution_failure,
                    audit_events,
                });
            }

            let prepared = match self.prepare(expected_state, action, requested) {
                Ok(prepared) => prepared,
                Err(failure) => {
                    let execution_failure = ActionExecutionFailure::preflight(failure);
                    audit_events.push(audit_without_prepared(
                        index,
                        requested,
                        expected_state,
                        audit_target,
                        audit_destination,
                        execution_failure.failure().class(),
                    ));
                    return Err(PreflightAbort {
                        index,
                        failure: execution_failure,
                        audit_events,
                    });
                }
            };

            if let Err(failure) = self.authorize_prepared(&prepared) {
                let execution_failure = ActionExecutionFailure::preflight(failure);
                audit_events.push(audit_from_prepared(
                    index,
                    &prepared,
                    BatchAuditPhase::PreflightRejected,
                    Some(execution_failure.failure().class()),
                    Some(SideEffectStatus::NotStarted),
                ));
                return Err(PreflightAbort {
                    index,
                    failure: execution_failure,
                    audit_events,
                });
            }
            audit_events.push(audit_from_prepared(
                index,
                &prepared,
                BatchAuditPhase::Authorized,
                None,
                None,
            ));

            match self.simulate_prepared(requested, expected_state, &prepared) {
                Ok(true) => {}
                Ok(false) => crossed_network_boundary = true,
                Err(failure) => {
                    audit_events.push(audit_from_prepared(
                        index,
                        &prepared,
                        BatchAuditPhase::PreflightRejected,
                        Some(failure.failure().class()),
                        Some(SideEffectStatus::NotStarted),
                    ));
                    return Err(PreflightAbort {
                        index,
                        failure,
                        audit_events,
                    });
                }
            }
            prepared_items.push(PreflightItem {
                requested,
                expected_state,
                prepared,
            });
        }
        Ok((prepared_items, audit_events))
    }
}

fn audit_from_prepared(
    index: usize,
    prepared: &Prepared,
    phase: BatchAuditPhase,
    failure_class: Option<mawr_core::FailureClass>,
    side_effect: Option<SideEffectStatus>,
) -> BatchAuditEvent {
    let authorization = &prepared.authorization;
    BatchAuditEvent::new(
        index,
        authorization.requested(),
        Some(authorization.effective()),
        authorization.expected_state(),
        authorization.operation(),
        authorization.target(),
        authorization.destination().cloned(),
        phase,
        failure_class,
        side_effect,
    )
}

fn audit_without_prepared(
    index: usize,
    requested: ActionKind,
    expected_state: StateId,
    target: Option<mawr_core::ElementRef>,
    destination: Option<mawr_core::AbsoluteUrl>,
    failure_class: mawr_core::FailureClass,
) -> BatchAuditEvent {
    BatchAuditEvent::new(
        index,
        requested,
        None,
        expected_state,
        OperationKind::Act(requested),
        target,
        destination,
        BatchAuditPhase::PreflightRejected,
        Some(failure_class),
        Some(SideEffectStatus::NotStarted),
    )
}

fn unprepared_context(
    action: &Action,
) -> (
    Option<mawr_core::ElementRef>,
    Option<mawr_core::AbsoluteUrl>,
) {
    match action {
        Action::Navigate(destination) => (None, Some(destination.clone())),
        Action::Follow(target)
        | Action::Check(target)
        | Action::Uncheck(target)
        | Action::Submit(target)
        | Action::Fill { target, .. }
        | Action::Select { target, .. } => (Some(*target), None),
        Action::Press { target, .. } => (*target, None),
    }
}

fn micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}
