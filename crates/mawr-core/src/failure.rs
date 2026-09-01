use std::num::NonZeroU64;

use crate::{
    ActionKind, Capability, ElementRef, EngineIdentity, NonEmptyText, StateId, UnsupportedReason,
    ValidationError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperationKind {
    Observe,
    Navigate,
    Download,
    Act(ActionKind),
    ReadState,
    StartEngine,
    Parse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthorizationReason {
    DestinationDenied,
    MutationNotGranted,
    SessionDenied,
    ConfirmationRequired,
    PolicyDenied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceKind {
    ResponseBytes,
    ResponseHeaders,
    DownloadBytes,
    DnsAddresses,
    FormBytes,
    SessionCookies,
    DomNodes,
    DomDepth,
    HtmlAttributes,
    DocumentTextBytes,
    SemanticUnits,
    SemanticRelationships,
    ExtractionNotices,
    StateRetention,
    ObservationTokens,
    Actions,
    CpuTime,
    WallTime,
    Memory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NavigationFailureKind {
    InvalidDestination,
    RedirectLoop,
    TooManyRedirects,
    MissingRedirectLocation,
    Dns,
    Connection,
    SecureConnection,
    Response,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProtocolFailureKind {
    InvalidMessage,
    VersionMismatch,
    TruncatedMessage,
    PeerDisconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParsingFailureKind {
    InvalidDocument,
    DepthLimit,
    NodeLimit,
    UnsupportedEncoding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EngineFailureKind {
    Startup,
    Crashed,
    CapabilityMismatch,
    StateUnavailable,
    Execution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdapterFailureKind {
    InvalidAdapterData,
    ExternalProcess,
    Transport,
    Translation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetryDisposition {
    Never,
    AfterObservation,
    AfterConfigurationChange,
    Transient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FailureClass {
    InvalidInput,
    UnsupportedCapability,
    MissingReference,
    StaleState,
    AuthorizationDenied,
    ResourceLimit,
    Navigation,
    Protocol,
    Parsing,
    Engine,
    Adapter,
    Timeout,
    Cancelled,
    InvariantViolation,
}

impl FailureClass {
    pub const COUNT: usize = 14;
    pub const ALL: [Self; Self::COUNT] = [
        Self::InvalidInput,
        Self::UnsupportedCapability,
        Self::MissingReference,
        Self::StaleState,
        Self::AuthorizationDenied,
        Self::ResourceLimit,
        Self::Navigation,
        Self::Protocol,
        Self::Parsing,
        Self::Engine,
        Self::Adapter,
        Self::Timeout,
        Self::Cancelled,
        Self::InvariantViolation,
    ];
}

/// Structured operational failure with no free-form engine message or secret
/// value in its representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationFailure {
    InvalidInput(ValidationError),
    UnsupportedCapability {
        capability: Capability,
        engine: EngineIdentity,
        reason: UnsupportedReason,
    },
    MissingReference {
        reference: ElementRef,
    },
    StaleState {
        expected: StateId,
        actual: Option<StateId>,
    },
    AuthorizationDenied {
        operation: OperationKind,
        reason: AuthorizationReason,
    },
    ResourceLimit {
        resource: ResourceKind,
        configured_limit: NonZeroU64,
    },
    NavigationFailure(NavigationFailureKind),
    ProtocolFailure(ProtocolFailureKind),
    ParsingFailure(ParsingFailureKind),
    EngineFailure {
        engine: EngineIdentity,
        kind: EngineFailureKind,
    },
    AdapterFailure {
        engine: EngineIdentity,
        kind: AdapterFailureKind,
    },
    Timeout {
        operation: OperationKind,
        limit_millis: NonZeroU64,
    },
    Cancelled {
        operation: OperationKind,
    },
    InvariantViolation {
        code: NonEmptyText<64>,
    },
}

impl OperationFailure {
    #[must_use]
    pub const fn class(&self) -> FailureClass {
        match self {
            Self::InvalidInput(_) => FailureClass::InvalidInput,
            Self::UnsupportedCapability { .. } => FailureClass::UnsupportedCapability,
            Self::MissingReference { .. } => FailureClass::MissingReference,
            Self::StaleState { .. } => FailureClass::StaleState,
            Self::AuthorizationDenied { .. } => FailureClass::AuthorizationDenied,
            Self::ResourceLimit { .. } => FailureClass::ResourceLimit,
            Self::NavigationFailure(_) => FailureClass::Navigation,
            Self::ProtocolFailure(_) => FailureClass::Protocol,
            Self::ParsingFailure(_) => FailureClass::Parsing,
            Self::EngineFailure { .. } => FailureClass::Engine,
            Self::AdapterFailure { .. } => FailureClass::Adapter,
            Self::Timeout { .. } => FailureClass::Timeout,
            Self::Cancelled { .. } => FailureClass::Cancelled,
            Self::InvariantViolation { .. } => FailureClass::InvariantViolation,
        }
    }

    #[must_use]
    pub const fn retry_disposition(&self) -> RetryDisposition {
        match self {
            Self::MissingReference { .. } | Self::StaleState { .. } => {
                RetryDisposition::AfterObservation
            }
            Self::UnsupportedCapability { .. }
            | Self::AuthorizationDenied { .. }
            | Self::ResourceLimit { .. } => RetryDisposition::AfterConfigurationChange,
            Self::NavigationFailure(_)
            | Self::EngineFailure { .. }
            | Self::AdapterFailure { .. }
            | Self::Timeout { .. } => RetryDisposition::Transient,
            Self::InvalidInput(_)
            | Self::ProtocolFailure(_)
            | Self::ParsingFailure(_)
            | Self::Cancelled { .. }
            | Self::InvariantViolation { .. } => RetryDisposition::Never,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::num::NonZeroU64;

    use crate::{
        Capability, ElementRef, EngineIdentity, EngineKind, NonEmptyText, SessionId, StateId,
        UnsupportedReason, ValidationError, ValidationIssue,
    };

    use super::{
        AdapterFailureKind, AuthorizationReason, EngineFailureKind, FailureClass,
        NavigationFailureKind, OperationFailure, OperationKind, ParsingFailureKind,
        ProtocolFailureKind, ResourceKind, RetryDisposition,
    };

    #[test]
    fn failure_class_vocabulary_is_unique_and_counted() {
        assert_eq!(
            FailureClass::ALL.into_iter().collect::<BTreeSet<_>>().len(),
            FailureClass::COUNT
        );
    }

    #[test]
    fn every_structured_failure_maps_to_one_declared_class_and_retry_policy() {
        let session = SessionId::new(1).unwrap();
        let state = StateId::new(session, 1).unwrap();
        let engine = EngineIdentity::new("native-static", "0", EngineKind::NativeStatic).unwrap();
        let one = NonZeroU64::new(1).unwrap();
        let failures = [
            (
                OperationFailure::InvalidInput(ValidationError::new(
                    "subject",
                    ValidationIssue::Empty,
                )),
                FailureClass::InvalidInput,
                RetryDisposition::Never,
            ),
            (
                OperationFailure::UnsupportedCapability {
                    capability: Capability::JavaScript,
                    engine: engine.clone(),
                    reason: UnsupportedReason::EngineLimitation,
                },
                FailureClass::UnsupportedCapability,
                RetryDisposition::AfterConfigurationChange,
            ),
            (
                OperationFailure::MissingReference {
                    reference: ElementRef::new(session, 1).unwrap(),
                },
                FailureClass::MissingReference,
                RetryDisposition::AfterObservation,
            ),
            (
                OperationFailure::StaleState {
                    expected: state,
                    actual: None,
                },
                FailureClass::StaleState,
                RetryDisposition::AfterObservation,
            ),
            (
                OperationFailure::AuthorizationDenied {
                    operation: OperationKind::Observe,
                    reason: AuthorizationReason::PolicyDenied,
                },
                FailureClass::AuthorizationDenied,
                RetryDisposition::AfterConfigurationChange,
            ),
            (
                OperationFailure::ResourceLimit {
                    resource: ResourceKind::SemanticUnits,
                    configured_limit: one,
                },
                FailureClass::ResourceLimit,
                RetryDisposition::AfterConfigurationChange,
            ),
            (
                OperationFailure::NavigationFailure(NavigationFailureKind::Connection),
                FailureClass::Navigation,
                RetryDisposition::Transient,
            ),
            (
                OperationFailure::ProtocolFailure(ProtocolFailureKind::InvalidMessage),
                FailureClass::Protocol,
                RetryDisposition::Never,
            ),
            (
                OperationFailure::ParsingFailure(ParsingFailureKind::InvalidDocument),
                FailureClass::Parsing,
                RetryDisposition::Never,
            ),
            (
                OperationFailure::EngineFailure {
                    engine: engine.clone(),
                    kind: EngineFailureKind::Execution,
                },
                FailureClass::Engine,
                RetryDisposition::Transient,
            ),
            (
                OperationFailure::AdapterFailure {
                    engine,
                    kind: AdapterFailureKind::Translation,
                },
                FailureClass::Adapter,
                RetryDisposition::Transient,
            ),
            (
                OperationFailure::Timeout {
                    operation: OperationKind::Observe,
                    limit_millis: one,
                },
                FailureClass::Timeout,
                RetryDisposition::Transient,
            ),
            (
                OperationFailure::Cancelled {
                    operation: OperationKind::Observe,
                },
                FailureClass::Cancelled,
                RetryDisposition::Never,
            ),
            (
                OperationFailure::InvariantViolation {
                    code: NonEmptyText::new("unexpected_state", "code").unwrap(),
                },
                FailureClass::InvariantViolation,
                RetryDisposition::Never,
            ),
        ];

        assert_eq!(failures.len(), FailureClass::COUNT);
        for (failure, expected_class, expected_retry) in failures {
            assert_eq!(failure.class(), expected_class);
            assert_eq!(failure.retry_disposition(), expected_retry);
        }
    }
}
