use crate::{ActionKind, EngineIdentity, PageIdentity, StateId, ValidationError, ValidationIssue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResetReason {
    BaseUnavailable,
    BaseEvicted,
    NavigationBoundary,
    AmbiguousIdentity,
    ExplicitRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransitionCause {
    Initial,
    Navigation,
    Action(ActionKind),
    Refresh,
    Reset(ResetReason),
    ExternalChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransition {
    from: Option<StateId>,
    to: StateId,
    page: PageIdentity,
    engine: EngineIdentity,
    cause: TransitionCause,
}

impl StateTransition {
    pub fn new(
        from: Option<StateId>,
        to: StateId,
        page: PageIdentity,
        engine: EngineIdentity,
        cause: TransitionCause,
    ) -> Result<Self, ValidationError> {
        if (from.is_none()
            && !matches!(
                cause,
                TransitionCause::Initial | TransitionCause::Navigation
            ))
            || (from.is_some() && cause == TransitionCause::Initial)
        {
            return Err(ValidationError::new(
                "transition_cause",
                ValidationIssue::InvalidTransition,
            ));
        }
        if let Some(from) = from {
            if from.session() != to.session() {
                return Err(session_mismatch("transition_from", to, from));
            }
            if from == to {
                return Err(ValidationError::new(
                    "state_transition",
                    ValidationIssue::InvalidTransition,
                ));
            }
        }
        if page.id().session() != to.session() {
            return Err(ValidationError::new(
                "transition_page",
                ValidationIssue::SessionMismatch {
                    expected: to.session().get(),
                    actual: page.id().session().get(),
                },
            ));
        }
        Ok(Self {
            from,
            to,
            page,
            engine,
            cause,
        })
    }

    #[must_use]
    pub const fn from(&self) -> Option<StateId> {
        self.from
    }

    #[must_use]
    pub const fn to(&self) -> StateId {
        self.to
    }

    #[must_use]
    pub const fn page(&self) -> &PageIdentity {
        &self.page
    }

    #[must_use]
    pub const fn engine(&self) -> &EngineIdentity {
        &self.engine
    }

    #[must_use]
    pub const fn cause(&self) -> TransitionCause {
        self.cause
    }
}

fn session_mismatch(field: &'static str, expected: StateId, actual: StateId) -> ValidationError {
    ValidationError::new(
        field,
        ValidationIssue::SessionMismatch {
            expected: expected.session().get(),
            actual: actual.session().get(),
        },
    )
}

#[cfg(test)]
mod tests {
    use crate::{AbsoluteUrl, EngineKind, PageId, SessionId};

    use super::*;

    fn page(session: SessionId) -> PageIdentity {
        PageIdentity::new(
            PageId::new(session, 1).unwrap(),
            AbsoluteUrl::new("https://example.test").unwrap(),
        )
    }

    fn engine() -> EngineIdentity {
        EngineIdentity::new("native-static", "0", EngineKind::NativeStatic).unwrap()
    }

    #[test]
    fn initial_transition_has_no_predecessor() {
        let session = SessionId::new(1).unwrap();
        let initial = StateTransition::new(
            None,
            StateId::new(session, 1).unwrap(),
            page(session),
            engine(),
            TransitionCause::Initial,
        );
        assert!(initial.is_ok());

        let invalid = StateTransition::new(
            Some(StateId::new(session, 1).unwrap()),
            StateId::new(session, 2).unwrap(),
            page(session),
            engine(),
            TransitionCause::Initial,
        );
        assert!(invalid.is_err());
    }

    #[test]
    fn transitions_reject_equal_or_cross_session_states() {
        let first = SessionId::new(1).unwrap();
        let second = SessionId::new(2).unwrap();
        let state = StateId::new(first, 1).unwrap();

        assert!(
            StateTransition::new(
                Some(state),
                state,
                page(first),
                engine(),
                TransitionCause::Refresh,
            )
            .is_err()
        );
        assert!(
            StateTransition::new(
                Some(state),
                StateId::new(second, 2).unwrap(),
                page(second),
                engine(),
                TransitionCause::Refresh,
            )
            .is_err()
        );
    }
}
