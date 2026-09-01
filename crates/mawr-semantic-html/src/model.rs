use std::fmt;
use std::num::NonZeroU32;

use mawr_core::{
    AbsoluteUrl, ActionAffordances, BoundedText, ElementState, MeasurementSet, Property,
    Provenance, RelationshipKind, SemanticRole, SemanticValue, SessionId,
};

pub const MAX_NAME_BYTES: usize = 512;
pub const MAX_DESCRIPTION_BYTES: usize = 2_048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceNodeId(NonZeroU32);

impl SourceNodeId {
    pub(crate) fn new(value: u32) -> Self {
        Self(NonZeroU32::new(value).expect("source node sequence is non-zero"))
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RoleOrigin {
    NativeHtml,
    ExplicitAria,
    EngineDerived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExtractionNoticeKind {
    DecodeReplacement,
    UnsupportedExplicitRole,
    DuplicateHtmlId,
    BrokenIdReference,
    CyclicNameReference,
    InvalidUrl,
    UnsupportedUrlScheme,
    ExternalCssVisibilityUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtractionNotice {
    kind: ExtractionNoticeKind,
    source: Option<SourceNodeId>,
}

impl ExtractionNotice {
    pub(crate) const fn new(kind: ExtractionNoticeKind, source: Option<SourceNodeId>) -> Self {
        Self { kind, source }
    }

    #[must_use]
    pub const fn kind(self) -> ExtractionNoticeKind {
        self.kind
    }

    #[must_use]
    pub const fn source(self) -> Option<SourceNodeId> {
        self.source
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtractedRelationship {
    kind: RelationshipKind,
    target: SourceNodeId,
}

impl ExtractedRelationship {
    pub(crate) const fn new(kind: RelationshipKind, target: SourceNodeId) -> Self {
        Self { kind, target }
    }

    #[must_use]
    pub const fn kind(self) -> RelationshipKind {
        self.kind
    }

    #[must_use]
    pub const fn target(self) -> SourceNodeId {
        self.target
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ExtractedSemanticUnit {
    pub(crate) source: SourceNodeId,
    pub(crate) parent_source: Option<SourceNodeId>,
    pub(crate) role: SemanticRole,
    pub(crate) role_origin: RoleOrigin,
    pub(crate) provenance: Provenance,
    pub(crate) name: Property<BoundedText<MAX_NAME_BYTES>>,
    pub(crate) description: Property<BoundedText<MAX_DESCRIPTION_BYTES>>,
    pub(crate) value: SemanticValue,
    pub(crate) state: ElementState,
    pub(crate) relationships: Vec<ExtractedRelationship>,
    pub(crate) affordances: ActionAffordances,
    pub(crate) destination: Property<AbsoluteUrl>,
}

impl ExtractedSemanticUnit {
    #[must_use]
    pub const fn source(&self) -> SourceNodeId {
        self.source
    }
    #[must_use]
    pub const fn parent_source(&self) -> Option<SourceNodeId> {
        self.parent_source
    }
    #[must_use]
    pub const fn role(&self) -> SemanticRole {
        self.role
    }
    #[must_use]
    pub const fn role_origin(&self) -> RoleOrigin {
        self.role_origin
    }
    #[must_use]
    pub const fn provenance(&self) -> Provenance {
        self.provenance
    }
    #[must_use]
    pub const fn name(&self) -> &Property<BoundedText<MAX_NAME_BYTES>> {
        &self.name
    }
    #[must_use]
    pub const fn description(&self) -> &Property<BoundedText<MAX_DESCRIPTION_BYTES>> {
        &self.description
    }
    #[must_use]
    pub const fn value(&self) -> &SemanticValue {
        &self.value
    }
    #[must_use]
    pub const fn state(&self) -> &ElementState {
        &self.state
    }
    #[must_use]
    pub fn relationships(&self) -> &[ExtractedRelationship] {
        &self.relationships
    }
    #[must_use]
    pub const fn affordances(&self) -> &ActionAffordances {
        &self.affordances
    }
    #[must_use]
    pub const fn destination(&self) -> &Property<AbsoluteUrl> {
        &self.destination
    }
}

impl fmt::Debug for ExtractedSemanticUnit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExtractedSemanticUnit")
            .field("source", &self.source)
            .field("parent_source", &self.parent_source)
            .field("role", &self.role)
            .field("role_origin", &self.role_origin)
            .field("provenance", &self.provenance)
            .field("name", &"<web-content>")
            .field("value", &"<web-content>")
            .field("state", &self.state)
            .field("relationships", &self.relationships)
            .field("affordances", &self.affordances)
            .field("destination", &self.destination)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionDiagnostics {
    input_bytes: u64,
    dom_nodes: u64,
    semantic_units: u64,
    relationships: u64,
    notices: u64,
    measurements: MeasurementSet,
}

impl ExtractionDiagnostics {
    pub(crate) const fn new(
        input_bytes: u64,
        dom_nodes: u64,
        semantic_units: u64,
        relationships: u64,
        notices: u64,
        measurements: MeasurementSet,
    ) -> Self {
        Self {
            input_bytes,
            dom_nodes,
            semantic_units,
            relationships,
            notices,
            measurements,
        }
    }
    #[must_use]
    pub const fn input_bytes(&self) -> u64 {
        self.input_bytes
    }
    #[must_use]
    pub const fn dom_nodes(&self) -> u64 {
        self.dom_nodes
    }
    #[must_use]
    pub const fn semantic_units(&self) -> u64 {
        self.semantic_units
    }
    #[must_use]
    pub const fn relationships(&self) -> u64 {
        self.relationships
    }
    #[must_use]
    pub const fn notices(&self) -> u64 {
        self.notices
    }
    #[must_use]
    pub const fn measurements(&self) -> &MeasurementSet {
        &self.measurements
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SemanticDocument {
    pub(crate) session: SessionId,
    pub(crate) document_url: AbsoluteUrl,
    pub(crate) base_url: AbsoluteUrl,
    pub(crate) title: Option<BoundedText<MAX_NAME_BYTES>>,
    pub(crate) language: Option<BoundedText<128>>,
    pub(crate) units: Vec<ExtractedSemanticUnit>,
    pub(crate) notices: Vec<ExtractionNotice>,
    pub(crate) diagnostics: ExtractionDiagnostics,
}

impl SemanticDocument {
    #[must_use]
    pub const fn session(&self) -> SessionId {
        self.session
    }
    #[must_use]
    pub const fn document_url(&self) -> &AbsoluteUrl {
        &self.document_url
    }
    #[must_use]
    pub const fn base_url(&self) -> &AbsoluteUrl {
        &self.base_url
    }
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().map(BoundedText::as_str)
    }
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.language.as_ref().map(BoundedText::as_str)
    }
    #[must_use]
    pub fn units(&self) -> &[ExtractedSemanticUnit] {
        &self.units
    }
    #[must_use]
    pub fn notices(&self) -> &[ExtractionNotice] {
        &self.notices
    }
    #[must_use]
    pub const fn diagnostics(&self) -> &ExtractionDiagnostics {
        &self.diagnostics
    }
}

impl fmt::Debug for SemanticDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticDocument")
            .field("session", &self.session)
            .field("document_url", &self.document_url)
            .field("base_url", &self.base_url)
            .field("title", &self.title.as_ref().map(|_| "<web-content>"))
            .field("language", &self.language)
            .field("unit_count", &self.units.len())
            .field("notices", &self.notices)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}
