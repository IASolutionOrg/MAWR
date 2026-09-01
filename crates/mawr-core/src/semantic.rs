use std::collections::BTreeSet;

use crate::{ActionKind, BoundedText, ElementRef, NonEmptyText, ValidationError, ValidationIssue};

const MAX_SEMANTIC_NAME_BYTES: usize = 512;
const MAX_SEMANTIC_VALUE_BYTES: usize = 4_096;
const MAX_RELATIONSHIPS_PER_UNIT: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticRole {
    Page,
    Region,
    Heading,
    Text,
    Link,
    Form,
    Textbox,
    Checkbox,
    Radio,
    Select,
    Option,
    Button,
    Table,
    Row,
    Cell,
    List,
    ListItem,
    Alert,
}

impl SemanticRole {
    pub const COUNT: usize = 18;
    pub const ALL: [Self; Self::COUNT] = [
        Self::Page,
        Self::Region,
        Self::Heading,
        Self::Text,
        Self::Link,
        Self::Form,
        Self::Textbox,
        Self::Checkbox,
        Self::Radio,
        Self::Select,
        Self::Option,
        Self::Button,
        Self::Table,
        Self::Row,
        Self::Cell,
        Self::List,
        Self::ListItem,
        Self::Alert,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Provenance {
    UntrustedWebContent,
    EngineDerived,
    TrustedUserInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PropertyUnknownReason {
    NotExposed,
    Ambiguous,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Property<T> {
    NotApplicable,
    Known(T),
    Unknown(PropertyUnknownReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementState {
    disabled: Property<bool>,
    checked: Property<bool>,
    selected: Property<bool>,
    expanded: Property<bool>,
    required: Property<bool>,
    invalid: Property<bool>,
}

impl ElementState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            disabled: Property::NotApplicable,
            checked: Property::NotApplicable,
            selected: Property::NotApplicable,
            expanded: Property::NotApplicable,
            required: Property::NotApplicable,
            invalid: Property::NotApplicable,
        }
    }

    #[must_use]
    pub fn with_disabled(mut self, value: Property<bool>) -> Self {
        self.disabled = value;
        self
    }

    #[must_use]
    pub fn with_checked(mut self, value: Property<bool>) -> Self {
        self.checked = value;
        self
    }

    #[must_use]
    pub fn with_selected(mut self, value: Property<bool>) -> Self {
        self.selected = value;
        self
    }

    #[must_use]
    pub fn with_expanded(mut self, value: Property<bool>) -> Self {
        self.expanded = value;
        self
    }

    #[must_use]
    pub fn with_required(mut self, value: Property<bool>) -> Self {
        self.required = value;
        self
    }

    #[must_use]
    pub fn with_invalid(mut self, value: Property<bool>) -> Self {
        self.invalid = value;
        self
    }

    #[must_use]
    pub const fn disabled(&self) -> &Property<bool> {
        &self.disabled
    }

    #[must_use]
    pub const fn checked(&self) -> &Property<bool> {
        &self.checked
    }

    #[must_use]
    pub const fn selected(&self) -> &Property<bool> {
        &self.selected
    }

    #[must_use]
    pub const fn expanded(&self) -> &Property<bool> {
        &self.expanded
    }

    #[must_use]
    pub const fn required(&self) -> &Property<bool> {
        &self.required
    }

    #[must_use]
    pub const fn invalid(&self) -> &Property<bool> {
        &self.invalid
    }
}

impl Default for ElementState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticValue {
    Absent,
    Text(BoundedText<MAX_SEMANTIC_VALUE_BYTES>),
    Redacted,
    Unknown(NonEmptyText<128>),
}

impl SemanticValue {
    pub fn text(value: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self::Text(BoundedText::new(value, "semantic_value")?))
    }

    pub fn unknown(reason: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self::Unknown(NonEmptyText::new(
            reason,
            "semantic_value_unknown_reason",
        )?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RelationshipKind {
    LabelledBy,
    DescribedBy,
    Contains,
    Controls,
    OwnedBy,
    OptionOf,
    RowOf,
    CellOf,
    ListItemOf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Relationship {
    kind: RelationshipKind,
    target: ElementRef,
}

impl Relationship {
    #[must_use]
    pub const fn new(kind: RelationshipKind, target: ElementRef) -> Self {
        Self { kind, target }
    }

    #[must_use]
    pub const fn kind(self) -> RelationshipKind {
        self.kind
    }

    #[must_use]
    pub const fn target(self) -> ElementRef {
        self.target
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionAffordances(BTreeSet<ActionKind>);

impl ActionAffordances {
    #[must_use]
    pub fn with(mut self, action: ActionKind) -> Self {
        self.0.insert(action);
        self
    }

    #[must_use]
    pub fn contains(&self, action: ActionKind) -> bool {
        self.0.contains(&action)
    }

    pub fn iter(&self) -> impl Iterator<Item = ActionKind> + '_ {
        self.0.iter().copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticUnit {
    reference: ElementRef,
    role: SemanticRole,
    provenance: Provenance,
    name: Option<NonEmptyText<MAX_SEMANTIC_NAME_BYTES>>,
    value: SemanticValue,
    state: ElementState,
    relationships: BTreeSet<Relationship>,
    affordances: ActionAffordances,
}

impl SemanticUnit {
    #[must_use]
    pub fn new(reference: ElementRef, role: SemanticRole, provenance: Provenance) -> Self {
        Self {
            reference,
            role,
            provenance,
            name: None,
            value: SemanticValue::Absent,
            state: ElementState::new(),
            relationships: BTreeSet::new(),
            affordances: ActionAffordances::default(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Result<Self, ValidationError> {
        self.name = Some(NonEmptyText::new(name, "semantic_name")?);
        Ok(self)
    }

    #[must_use]
    pub fn with_value(mut self, value: SemanticValue) -> Self {
        self.value = value;
        self
    }

    #[must_use]
    pub fn with_state(mut self, state: ElementState) -> Self {
        self.state = state;
        self
    }

    pub fn with_relationship(
        mut self,
        relationship: Relationship,
    ) -> Result<Self, ValidationError> {
        if relationship.target().session() != self.reference.session() {
            return Err(ValidationError::new(
                "relationship_target",
                ValidationIssue::SessionMismatch {
                    expected: self.reference.session().get(),
                    actual: relationship.target().session().get(),
                },
            ));
        }
        if relationship.target() == self.reference || !self.relationships.insert(relationship) {
            return Err(ValidationError::new(
                "relationship_target",
                ValidationIssue::Duplicate,
            ));
        }
        if self.relationships.len() > MAX_RELATIONSHIPS_PER_UNIT {
            return Err(ValidationError::new(
                "semantic_relationships",
                ValidationIssue::OutOfRange {
                    min: 0,
                    max: MAX_RELATIONSHIPS_PER_UNIT as u64,
                    actual: self.relationships.len() as u64,
                },
            ));
        }
        Ok(self)
    }

    #[must_use]
    pub fn with_affordance(mut self, action: ActionKind) -> Self {
        self.affordances = self.affordances.with(action);
        self
    }

    #[must_use]
    pub const fn reference(&self) -> ElementRef {
        self.reference
    }

    #[must_use]
    pub const fn role(&self) -> SemanticRole {
        self.role
    }

    #[must_use]
    pub const fn provenance(&self) -> Provenance {
        self.provenance
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(NonEmptyText::as_str)
    }

    #[must_use]
    pub const fn value(&self) -> &SemanticValue {
        &self.value
    }

    #[must_use]
    pub const fn state(&self) -> &ElementState {
        &self.state
    }

    pub fn relationships(&self) -> impl Iterator<Item = Relationship> + '_ {
        self.relationships.iter().copied()
    }

    #[must_use]
    pub const fn affordances(&self) -> &ActionAffordances {
        &self.affordances
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::{ActionKind, ElementRef, SessionId};

    use super::{
        MAX_RELATIONSHIPS_PER_UNIT, Provenance, Relationship, RelationshipKind, SemanticRole,
        SemanticUnit,
    };

    #[test]
    fn semantic_roles_are_exhaustive_and_unique() {
        assert_eq!(
            SemanticRole::ALL.into_iter().collect::<BTreeSet<_>>().len(),
            SemanticRole::COUNT
        );
    }

    #[test]
    fn affordances_are_deterministic_sets() {
        let session = SessionId::new(1).unwrap();
        let unit = SemanticUnit::new(
            ElementRef::new(session, 1).unwrap(),
            SemanticRole::Link,
            Provenance::UntrustedWebContent,
        )
        .with_affordance(ActionKind::Follow)
        .with_affordance(ActionKind::Follow);
        assert_eq!(
            unit.affordances().iter().collect::<Vec<_>>(),
            [ActionKind::Follow]
        );
    }

    #[test]
    fn relationships_are_session_scoped_and_bounded() {
        let session = SessionId::new(1).unwrap();
        let mut unit = SemanticUnit::new(
            ElementRef::new(session, 1).unwrap(),
            SemanticRole::Textbox,
            Provenance::UntrustedWebContent,
        );
        for sequence in 2..=(MAX_RELATIONSHIPS_PER_UNIT as u32 + 1) {
            unit = unit
                .with_relationship(Relationship::new(
                    RelationshipKind::DescribedBy,
                    ElementRef::new(session, sequence).unwrap(),
                ))
                .unwrap();
        }

        assert_eq!(unit.relationships().count(), MAX_RELATIONSHIPS_PER_UNIT);
        assert!(
            unit.with_relationship(Relationship::new(
                RelationshipKind::DescribedBy,
                ElementRef::new(session, MAX_RELATIONSHIPS_PER_UNIT as u32 + 2).unwrap(),
            ))
            .is_err()
        );
    }
}
