use std::fmt;
use std::num::NonZeroU32;

use mawr_core::{
    AbsoluteUrl, ActionAffordances, ActionKind, BoundedText, ElementState, MeasurementSet,
    Property, Provenance, RelationshipKind, SemanticRole, SemanticValue, SensitiveText, SessionId,
};

pub const MAX_NAME_BYTES: usize = 512;
pub const MAX_DESCRIPTION_BYTES: usize = 2_048;
pub const MAX_AUTHOR_ID_BYTES: usize = 256;
pub const MAX_CONTROL_NAME_BYTES: usize = 256;
pub const MAX_CONTROL_VALUE_BYTES: usize = 16_384;

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
pub enum StaticInteractionKind {
    None,
    Link,
    Form,
    TextControl,
    FileControl,
    Checkbox,
    Radio,
    Select,
    Option,
    SubmitButton,
    ImageButton,
    ResetButton,
    Button,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StaticFormMethod {
    Get,
    Post,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StaticFormEncoding {
    UrlEncoded,
    Multipart,
    TextPlain,
    Unsupported,
}

#[derive(Clone, PartialEq, Eq)]
pub struct StaticInteraction {
    pub(crate) kind: StaticInteractionKind,
    pub(crate) owner: Option<SourceNodeId>,
    pub(crate) name: Option<BoundedText<MAX_CONTROL_NAME_BYTES>>,
    pub(crate) submission_value: Option<SensitiveText<MAX_CONTROL_VALUE_BYTES>>,
    pub(crate) method: Option<StaticFormMethod>,
    pub(crate) encoding: Option<StaticFormEncoding>,
    pub(crate) multiple: bool,
    pub(crate) password: bool,
    pub(crate) download: bool,
    pub(crate) no_validate: bool,
    pub(crate) supported: bool,
}

impl StaticInteraction {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            kind: StaticInteractionKind::None,
            owner: None,
            name: None,
            submission_value: None,
            method: None,
            encoding: None,
            multiple: false,
            password: false,
            download: false,
            no_validate: false,
            supported: true,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> StaticInteractionKind {
        self.kind
    }
    #[must_use]
    pub const fn owner(&self) -> Option<SourceNodeId> {
        self.owner
    }
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(BoundedText::as_str)
    }
    #[must_use]
    pub fn submission_value(&self) -> Option<&str> {
        self.submission_value
            .as_ref()
            .map(SensitiveText::expose_secret)
    }
    #[must_use]
    pub const fn method(&self) -> Option<StaticFormMethod> {
        self.method
    }
    #[must_use]
    pub const fn encoding(&self) -> Option<StaticFormEncoding> {
        self.encoding
    }
    #[must_use]
    pub const fn multiple(&self) -> bool {
        self.multiple
    }
    #[must_use]
    pub const fn password(&self) -> bool {
        self.password
    }
    #[must_use]
    pub const fn download(&self) -> bool {
        self.download
    }
    #[must_use]
    pub const fn no_validate(&self) -> bool {
        self.no_validate
    }
    #[must_use]
    pub const fn supported(&self) -> bool {
        self.supported
    }
}

impl Default for StaticInteraction {
    fn default() -> Self {
        Self::none()
    }
}

impl fmt::Debug for StaticInteraction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StaticInteraction")
            .field("kind", &self.kind)
            .field("owner", &self.owner)
            .field("has_name", &self.name.is_some())
            .field("has_submission_value", &self.submission_value.is_some())
            .field("method", &self.method)
            .field("encoding", &self.encoding)
            .field("multiple", &self.multiple)
            .field("password", &self.password)
            .field("download", &self.download)
            .field("no_validate", &self.no_validate)
            .field("supported", &self.supported)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct StaticHiddenControl {
    pub(crate) source: SourceNodeId,
    pub(crate) owner: SourceNodeId,
    pub(crate) name: Option<BoundedText<MAX_CONTROL_NAME_BYTES>>,
    pub(crate) value: Option<SensitiveText<MAX_CONTROL_VALUE_BYTES>>,
    pub(crate) supported: bool,
}

impl StaticHiddenControl {
    #[must_use]
    pub const fn source(&self) -> SourceNodeId {
        self.source
    }
    #[must_use]
    pub const fn owner(&self) -> SourceNodeId {
        self.owner
    }
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(BoundedText::as_str)
    }
    #[must_use]
    pub fn value(&self) -> Option<&str> {
        self.value.as_ref().map(SensitiveText::expose_secret)
    }
    #[must_use]
    pub const fn supported(&self) -> bool {
        self.supported
    }
}

impl fmt::Debug for StaticHiddenControl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StaticHiddenControl")
            .field("source", &self.source)
            .field("owner", &self.owner)
            .field("name", &"<web-content>")
            .field("value", &"<redacted>")
            .field("supported", &self.supported)
            .finish()
    }
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
    pub(crate) author_id: Property<BoundedText<MAX_AUTHOR_ID_BYTES>>,
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
    pub(crate) interaction: StaticInteraction,
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
    pub const fn author_id(&self) -> &Property<BoundedText<MAX_AUTHOR_ID_BYTES>> {
        &self.author_id
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
    #[must_use]
    pub const fn interaction(&self) -> &StaticInteraction {
        &self.interaction
    }
}

impl fmt::Debug for ExtractedSemanticUnit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExtractedSemanticUnit")
            .field("source", &self.source)
            .field("parent_source", &self.parent_source)
            .field("author_id", &"<web-content>")
            .field("role", &self.role)
            .field("role_origin", &self.role_origin)
            .field("provenance", &self.provenance)
            .field("name", &"<web-content>")
            .field("value", &"<web-content>")
            .field("state", &self.state)
            .field("relationships", &self.relationships)
            .field("affordances", &self.affordances)
            .field("destination", &self.destination)
            .field("interaction", &self.interaction)
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
    pub(crate) hidden_controls: Vec<StaticHiddenControl>,
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
    pub fn hidden_controls(&self) -> &[StaticHiddenControl] {
        &self.hidden_controls
    }
    #[must_use]
    pub fn notices(&self) -> &[ExtractionNotice] {
        &self.notices
    }
    #[must_use]
    pub const fn diagnostics(&self) -> &ExtractionDiagnostics {
        &self.diagnostics
    }

    /// Applies a validated static text mutation to the retained semantic model.
    /// Returns `false` if the source is not a supported native text control.
    pub fn fill_static_control(
        &mut self,
        source: SourceNodeId,
        value: SensitiveText<MAX_CONTROL_VALUE_BYTES>,
    ) -> bool {
        let Some(unit) = self.units.iter_mut().find(|unit| unit.source == source) else {
            return false;
        };
        if unit.interaction.kind != StaticInteractionKind::TextControl
            || !unit.interaction.supported
        {
            return false;
        }
        let empty = value.expose_secret().is_empty();
        unit.interaction.submission_value = Some(value.clone());
        unit.value = if unit.interaction.password {
            SemanticValue::Redacted
        } else {
            semantic_text(value.expose_secret())
        };
        if unit.state.required() == &Property::Known(true) {
            unit.state = unit.state.clone().with_invalid(Property::Known(empty));
        }
        true
    }

    /// Applies a validated static checkbox or radio mutation.
    pub fn set_static_checked(&mut self, source: SourceNodeId, checked: bool) -> bool {
        let Some(target) = self.units.iter().find(|unit| unit.source == source) else {
            return false;
        };
        let interaction = target.interaction.clone();
        if !interaction.supported
            || !matches!(
                interaction.kind,
                StaticInteractionKind::Checkbox | StaticInteractionKind::Radio
            )
            || interaction.kind == StaticInteractionKind::Radio && !checked
        {
            return false;
        }
        if interaction.kind == StaticInteractionKind::Radio {
            for unit in &mut self.units {
                if unit.interaction.kind == StaticInteractionKind::Radio
                    && unit.interaction.owner == interaction.owner
                    && unit.interaction.name == interaction.name
                {
                    unit.state = unit.state.clone().with_checked(Property::Known(false));
                }
            }
        }
        let unit = self
            .units
            .iter_mut()
            .find(|unit| unit.source == source)
            .expect("source was resolved before mutation");
        unit.state = unit.state.clone().with_checked(Property::Known(checked));
        if unit.state.required() == &Property::Known(true) {
            unit.state = unit.state.clone().with_invalid(Property::Known(!checked));
        }
        if interaction.kind == StaticInteractionKind::Checkbox {
            unit.affordances = unit
                .affordances
                .clone()
                .without(ActionKind::Check)
                .without(ActionKind::Uncheck)
                .with(if checked {
                    ActionKind::Uncheck
                } else {
                    ActionKind::Check
                });
        }
        true
    }

    /// Applies a validated native option selection.
    pub fn select_static_option(
        &mut self,
        select_source: SourceNodeId,
        option_source: SourceNodeId,
    ) -> bool {
        let Some(select) = self.units.iter().find(|unit| unit.source == select_source) else {
            return false;
        };
        let multiple = select.interaction.multiple;
        if select.interaction.kind != StaticInteractionKind::Select
            || !select.interaction.supported
            || !self.units.iter().any(|unit| {
                unit.source == option_source
                    && unit.interaction.kind == StaticInteractionKind::Option
                    && unit.interaction.owner == Some(select_source)
                    && unit.interaction.supported
            })
        {
            return false;
        }
        for unit in &mut self.units {
            if unit.interaction.kind == StaticInteractionKind::Option
                && unit.interaction.owner == Some(select_source)
                && (!multiple || unit.source == option_source)
            {
                unit.state = unit
                    .state
                    .clone()
                    .with_selected(Property::Known(unit.source == option_source));
            }
        }
        let values = self
            .units
            .iter()
            .filter(|unit| {
                unit.interaction.kind == StaticInteractionKind::Option
                    && unit.interaction.owner == Some(select_source)
                    && unit.state.selected() == &Property::Known(true)
            })
            .filter_map(|unit| unit.interaction.submission_value())
            .collect::<Vec<_>>()
            .join(" ");
        let select = self
            .units
            .iter_mut()
            .find(|unit| unit.source == select_source)
            .expect("select source was resolved before mutation");
        select.value = semantic_text(&values);
        if select.state.required() == &Property::Known(true) {
            select.state = select
                .state
                .clone()
                .with_invalid(Property::Known(values.is_empty()));
        }
        true
    }
}

fn semantic_text(value: &str) -> SemanticValue {
    if value.is_empty() {
        return SemanticValue::Absent;
    }
    let mut end = value.len().min(4_096);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    SemanticValue::text(&value[..end]).expect("semantic value was bounded")
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
            .field("hidden_control_count", &self.hidden_controls.len())
            .field("notices", &self.notices)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}
