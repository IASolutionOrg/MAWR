use crate::{AbsoluteUrl, ElementRef, SensitiveText, StateId, ValidationError, ValidationIssue};

const MAX_INPUT_VALUE_BYTES: usize = 16_384;
pub const MAX_ACTIONS_PER_BATCH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionKind {
    Navigate,
    Follow,
    Fill,
    Select,
    Check,
    Uncheck,
    Submit,
    Press,
}

impl ActionKind {
    pub const COUNT: usize = 8;
    pub const ALL: [Self; Self::COUNT] = [
        Self::Navigate,
        Self::Follow,
        Self::Fill,
        Self::Select,
        Self::Check,
        Self::Uncheck,
        Self::Submit,
        Self::Press,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PressCommand {
    Enter,
    Escape,
    Tab,
    Space,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
}

/// A typed action. It has no execution authority until wrapped in an
/// [`ActionRequest`] whose expected state validates every reference scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Navigate(AbsoluteUrl),
    Follow(ElementRef),
    Fill {
        target: ElementRef,
        value: SensitiveText<MAX_INPUT_VALUE_BYTES>,
    },
    Select {
        target: ElementRef,
        option: ElementRef,
    },
    Check(ElementRef),
    Uncheck(ElementRef),
    Submit(ElementRef),
    Press {
        target: Option<ElementRef>,
        command: PressCommand,
    },
}

impl Action {
    #[must_use]
    pub const fn navigate(url: AbsoluteUrl) -> Self {
        Self::Navigate(url)
    }

    #[must_use]
    pub const fn follow(target: ElementRef) -> Self {
        Self::Follow(target)
    }

    pub fn fill(target: ElementRef, value: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self::Fill {
            target,
            value: SensitiveText::new(value, "fill_value")?,
        })
    }

    pub fn select(target: ElementRef, option: ElementRef) -> Result<Self, ValidationError> {
        ensure_session("select_option", target, option)?;
        Ok(Self::Select { target, option })
    }

    #[must_use]
    pub const fn check(target: ElementRef) -> Self {
        Self::Check(target)
    }

    #[must_use]
    pub const fn uncheck(target: ElementRef) -> Self {
        Self::Uncheck(target)
    }

    #[must_use]
    pub const fn submit(target: ElementRef) -> Self {
        Self::Submit(target)
    }

    #[must_use]
    pub const fn press(target: Option<ElementRef>, command: PressCommand) -> Self {
        Self::Press { target, command }
    }

    #[must_use]
    pub const fn kind(&self) -> ActionKind {
        match self {
            Self::Navigate(_) => ActionKind::Navigate,
            Self::Follow(_) => ActionKind::Follow,
            Self::Fill { .. } => ActionKind::Fill,
            Self::Select { .. } => ActionKind::Select,
            Self::Check(_) => ActionKind::Check,
            Self::Uncheck(_) => ActionKind::Uncheck,
            Self::Submit(_) => ActionKind::Submit,
            Self::Press { .. } => ActionKind::Press,
        }
    }

    fn referenced_elements(&self) -> Vec<ElementRef> {
        match self {
            Self::Navigate(_) => Vec::new(),
            Self::Follow(target)
            | Self::Check(target)
            | Self::Uncheck(target)
            | Self::Submit(target)
            | Self::Fill { target, .. } => vec![*target],
            Self::Select { target, option } => vec![*target, *option],
            Self::Press { target, .. } => target.iter().copied().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRequest {
    expected_state: StateId,
    action: Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BatchFailurePolicy {
    StopOnFailure,
    ContinueIndependent,
}

/// An ordered, bounded action batch scoped to one observed state.
///
/// The expected state is checked before whole-batch preflight. Individual
/// actions receive the deterministic state produced by earlier local actions;
/// reference-bearing actions after a network boundary are rejected because
/// their resulting semantic state cannot be known before execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionBatch {
    expected_state: StateId,
    actions: Vec<Action>,
    failure_policy: BatchFailurePolicy,
}

impl ActionBatch {
    pub fn new(
        expected_state: StateId,
        actions: Vec<Action>,
        failure_policy: BatchFailurePolicy,
    ) -> Result<Self, ValidationError> {
        if actions.is_empty() {
            return Err(ValidationError::new(
                "batch_actions",
                ValidationIssue::Empty,
            ));
        }
        if actions.len() > MAX_ACTIONS_PER_BATCH {
            return Err(ValidationError::new(
                "batch_actions",
                ValidationIssue::OutOfRange {
                    min: 1,
                    max: MAX_ACTIONS_PER_BATCH as u64,
                    actual: actions.len() as u64,
                },
            ));
        }
        for action in &actions {
            for reference in action.referenced_elements() {
                if reference.session() != expected_state.session() {
                    return Err(ValidationError::new(
                        "batch_action_reference",
                        ValidationIssue::SessionMismatch {
                            expected: expected_state.session().get(),
                            actual: reference.session().get(),
                        },
                    ));
                }
            }
        }
        Ok(Self {
            expected_state,
            actions,
            failure_policy,
        })
    }

    #[must_use]
    pub const fn expected_state(&self) -> StateId {
        self.expected_state
    }

    #[must_use]
    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    #[must_use]
    pub const fn failure_policy(&self) -> BatchFailurePolicy {
        self.failure_policy
    }
}

impl ActionRequest {
    pub fn new(expected_state: StateId, action: Action) -> Result<Self, ValidationError> {
        for reference in action.referenced_elements() {
            if reference.session() != expected_state.session() {
                return Err(ValidationError::new(
                    "action_reference",
                    ValidationIssue::SessionMismatch {
                        expected: expected_state.session().get(),
                        actual: reference.session().get(),
                    },
                ));
            }
        }
        Ok(Self {
            expected_state,
            action,
        })
    }

    #[must_use]
    pub const fn expected_state(&self) -> StateId {
        self.expected_state
    }

    #[must_use]
    pub const fn action(&self) -> &Action {
        &self.action
    }
}

fn ensure_session(
    field: &'static str,
    expected: ElementRef,
    actual: ElementRef,
) -> Result<(), ValidationError> {
    if expected.session() != actual.session() {
        return Err(ValidationError::new(
            field,
            ValidationIssue::SessionMismatch {
                expected: expected.session().get(),
                actual: actual.session().get(),
            },
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{ElementRef, SessionId, StateId};

    use super::{Action, ActionBatch, ActionRequest, BatchFailurePolicy, MAX_ACTIONS_PER_BATCH};

    #[test]
    fn request_rejects_cross_session_references() {
        let expected_session = SessionId::new(1).unwrap();
        let other_session = SessionId::new(2).unwrap();
        let state = StateId::new(expected_session, 1).unwrap();
        let action = Action::follow(ElementRef::new(other_session, 1).unwrap());

        assert!(ActionRequest::new(state, action).is_err());
    }

    #[test]
    fn action_debug_never_exposes_fill_value() {
        let session = SessionId::new(1).unwrap();
        let action = Action::fill(ElementRef::new(session, 1).unwrap(), "secret-value").unwrap();
        assert!(!format!("{action:?}").contains("secret-value"));
    }

    #[test]
    fn batches_are_nonempty_bounded_session_scoped_and_secret_safe() {
        let session = SessionId::new(1).unwrap();
        let foreign = SessionId::new(2).unwrap();
        let state = StateId::new(session, 1).unwrap();
        assert!(ActionBatch::new(state, Vec::new(), BatchFailurePolicy::StopOnFailure).is_err());
        assert!(
            ActionBatch::new(
                state,
                vec![
                    Action::navigate(crate::AbsoluteUrl::new("https://example.test/").unwrap());
                    MAX_ACTIONS_PER_BATCH + 1
                ],
                BatchFailurePolicy::StopOnFailure,
            )
            .is_err()
        );
        assert!(
            ActionBatch::new(
                state,
                vec![Action::follow(ElementRef::new(foreign, 1).unwrap())],
                BatchFailurePolicy::StopOnFailure,
            )
            .is_err()
        );
        let batch = ActionBatch::new(
            state,
            vec![Action::fill(ElementRef::new(session, 1).unwrap(), "batch-secret").unwrap()],
            BatchFailurePolicy::ContinueIndependent,
        )
        .unwrap();
        assert!(!format!("{batch:?}").contains("batch-secret"));
    }
}
