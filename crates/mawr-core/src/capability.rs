use std::array;
use std::num::{NonZeroU32, NonZeroU64};

use crate::{EngineIdentity, NonEmptyText, ValidationError, ValidationIssue};

const MAX_CAPABILITY_CONSTRAINTS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Capability {
    Navigation,
    Http,
    Https,
    Redirects,
    HtmlParsing,
    SemanticContent,
    FormGet,
    FormPost,
    TextInput,
    Checkbox,
    Radio,
    Select,
    Button,
    Downloads,
    SessionCookies,
    PersistentStorage,
    JavaScript,
    Layout,
    Geometry,
    VisualRendering,
    NetworkObservation,
    KeyInput,
}

impl Capability {
    pub const COUNT: usize = 22;
    pub const ALL: [Self; Self::COUNT] = [
        Self::Navigation,
        Self::Http,
        Self::Https,
        Self::Redirects,
        Self::HtmlParsing,
        Self::SemanticContent,
        Self::FormGet,
        Self::FormPost,
        Self::TextInput,
        Self::Checkbox,
        Self::Radio,
        Self::Select,
        Self::Button,
        Self::Downloads,
        Self::SessionCookies,
        Self::PersistentStorage,
        Self::JavaScript,
        Self::Layout,
        Self::Geometry,
        Self::VisualRendering,
        Self::NetworkObservation,
        Self::KeyInput,
    ];

    const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnsupportedReason {
    NotImplemented,
    EngineLimitation,
    PolicyRestricted,
    ConfigurationDisabled,
    PlatformUnavailable,
    ResourceUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapabilityConstraint {
    MaxBytes(NonZeroU64),
    MaxOperations(NonZeroU32),
    SameOriginOnly,
    SessionOnly,
    NoPersistence,
    Other(NonEmptyText<128>),
}

/// A non-empty, deterministically ordered capability constraint set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityConstraints(Vec<CapabilityConstraint>);

impl CapabilityConstraints {
    #[must_use]
    pub fn new(first: CapabilityConstraint) -> Self {
        Self(vec![first])
    }

    pub fn with(mut self, constraint: CapabilityConstraint) -> Result<Self, ValidationError> {
        if !self.0.contains(&constraint) {
            if self.0.len() >= MAX_CAPABILITY_CONSTRAINTS {
                return Err(ValidationError::new(
                    "capability_constraints",
                    ValidationIssue::OutOfRange {
                        min: 1,
                        max: MAX_CAPABILITY_CONSTRAINTS as u64,
                        actual: self.0.len() as u64 + 1,
                    },
                ));
            }
            self.0.push(constraint);
            self.0.sort();
        }
        Ok(self)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[CapabilityConstraint] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityStatus {
    Supported,
    Unsupported(UnsupportedReason),
    Limited(CapabilityConstraints),
}

/// An exhaustive report: every known capability always has an explicit status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityReport {
    engine: EngineIdentity,
    statuses: [CapabilityStatus; Capability::COUNT],
}

impl CapabilityReport {
    #[must_use]
    pub fn unsupported_all(engine: EngineIdentity, reason: UnsupportedReason) -> Self {
        Self {
            engine,
            statuses: array::from_fn(|_| CapabilityStatus::Unsupported(reason)),
        }
    }

    #[must_use]
    pub fn with(mut self, capability: Capability, status: CapabilityStatus) -> Self {
        self.statuses[capability.index()] = status;
        self
    }

    #[must_use]
    pub const fn engine(&self) -> &EngineIdentity {
        &self.engine
    }

    #[must_use]
    pub const fn status(&self, capability: Capability) -> &CapabilityStatus {
        &self.statuses[capability.index()]
    }

    pub fn iter(&self) -> impl Iterator<Item = (Capability, &CapabilityStatus)> {
        Capability::ALL
            .into_iter()
            .map(|capability| (capability, self.status(capability)))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::{EngineIdentity, EngineKind, NonEmptyText};

    use super::{
        Capability, CapabilityConstraint, CapabilityConstraints, CapabilityReport,
        CapabilityStatus, MAX_CAPABILITY_CONSTRAINTS, UnsupportedReason,
    };

    #[test]
    fn all_capabilities_are_unique_and_reported() {
        assert_eq!(
            Capability::ALL.into_iter().collect::<BTreeSet<_>>().len(),
            Capability::COUNT
        );
        let engine = EngineIdentity::new("native-static", "0", EngineKind::NativeStatic).unwrap();
        let report = CapabilityReport::unsupported_all(engine, UnsupportedReason::NotImplemented)
            .with(Capability::HtmlParsing, CapabilityStatus::Supported);

        assert_eq!(report.iter().count(), Capability::COUNT);
        assert_eq!(
            report.status(Capability::HtmlParsing),
            &CapabilityStatus::Supported
        );
        assert_eq!(
            report.status(Capability::JavaScript),
            &CapabilityStatus::Unsupported(UnsupportedReason::NotImplemented)
        );
    }

    #[test]
    fn capability_constraints_are_bounded() {
        let constraint = |index| {
            CapabilityConstraint::Other(
                NonEmptyText::new(format!("constraint-{index}"), "constraint").unwrap(),
            )
        };
        let mut constraints = CapabilityConstraints::new(constraint(0));
        for index in 1..MAX_CAPABILITY_CONSTRAINTS {
            constraints = constraints.with(constraint(index)).unwrap();
        }

        assert_eq!(constraints.as_slice().len(), MAX_CAPABILITY_CONSTRAINTS);
        assert!(
            constraints
                .with(constraint(MAX_CAPABILITY_CONSTRAINTS))
                .is_err()
        );
    }
}
