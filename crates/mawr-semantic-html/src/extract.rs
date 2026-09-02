use std::collections::{HashMap, HashSet};
use std::time::Instant;

use ego_tree::{NodeId, NodeRef};
use mawr_core::{
    AbsoluteUrl, ActionAffordances, ActionKind, BoundedText, ElementState, Measurement,
    MeasurementKind, MeasurementSet, MeasurementSource, OperationFailure, Property,
    PropertyUnknownReason, Provenance, RelationshipKind, ResourceKind, SemanticRole, SemanticValue,
    UnavailableReason,
};
use url::Url;

use crate::HtmlDocumentSource;
use crate::config::ExtractionLimits;
use crate::decode;
use crate::dom::{ElementRef, Html, Node};
use crate::model::{
    ExtractedRelationship, ExtractedSemanticUnit, ExtractionDiagnostics, ExtractionNotice,
    ExtractionNoticeKind, MAX_AUTHOR_ID_BYTES, MAX_DESCRIPTION_BYTES, MAX_NAME_BYTES, RoleOrigin,
    SemanticDocument, SourceNodeId,
};
use crate::normalize::{bounded_optional, known_name, normalize, truncate};
use crate::roles::{input_role, is_labelable, name_from_content, semantic_role};
use crate::state::{element_state, option_selected};
use crate::tree::{first_element, hidden, nearest_element_parent, visible_text};

pub(crate) fn extract(
    source: HtmlDocumentSource<'_>,
    limits: &ExtractionLimits,
) -> Result<SemanticDocument, OperationFailure> {
    let started = Instant::now();
    if source.content_type.is_some_and(|content_type| {
        !matches!(
            content_type
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase()
                .as_str(),
            "text/html" | "application/xhtml+xml"
        )
    }) {
        return Err(OperationFailure::ParsingFailure(
            mawr_core::ParsingFailureKind::InvalidDocument,
        ));
    }
    enforce(
        source.bytes.len() as u64,
        limits.document_bytes(),
        ResourceKind::ResponseBytes,
    )?;

    let decoded = decode::decode(source.bytes, source.content_type);
    let document = Html::parse_document(&decoded.text);
    let inventory = Inventory::build(&document, limits)?;
    let mut notices = Vec::new();
    if decoded.had_replacements {
        push_notice(
            &mut notices,
            ExtractionNotice::new(ExtractionNoticeKind::DecodeReplacement, None),
            limits,
        )?;
    }
    if let Some(stylesheet) = document
        .tree
        .root()
        .descendants()
        .filter_map(ElementRef::wrap)
        .find(|element| {
            element.value().name() == "style"
                || element.value().name() == "link"
                    && element.attr("rel").is_some_and(|value| {
                        value
                            .split_ascii_whitespace()
                            .any(|token| token.eq_ignore_ascii_case("stylesheet"))
                    })
        })
    {
        push_notice(
            &mut notices,
            ExtractionNotice::new(
                ExtractionNoticeKind::ExternalCssVisibilityUnknown,
                Some(inventory.source(stylesheet.id())),
            ),
            limits,
        )?;
    }

    let parsed_url = Url::parse(source.document_url.as_str()).map_err(|_| {
        OperationFailure::ParsingFailure(mawr_core::ParsingFailureKind::InvalidDocument)
    })?;
    let base_url = resolve_base_url(&document, &parsed_url, &inventory, &mut notices, limits)?;
    let mut context = Context {
        inventory: &inventory,
        notices: &mut notices,
        limits,
    };

    let title_text = first_element(&document, "title")
        .and_then(|element| bounded_optional::<MAX_NAME_BYTES>(&visible_text(*element, false)));
    let language = document
        .root_element()
        .attr("lang")
        .and_then(|value| bounded_optional::<128>(&normalize(value)));

    let mut units = Vec::new();
    let page_name = title_text
        .clone()
        .map_or(Property::NotApplicable, Property::Known);
    units.push(ExtractedSemanticUnit {
        source: SourceNodeId::new(1),
        parent_source: None,
        author_id: Property::NotApplicable,
        role: SemanticRole::Page,
        role_origin: RoleOrigin::EngineDerived,
        provenance: Provenance::EngineDerived,
        name: page_name,
        description: Property::NotApplicable,
        value: SemanticValue::Absent,
        state: ElementState::new(),
        relationships: Vec::new(),
        affordances: ActionAffordances::default(),
        destination: Property::Known(source.document_url.clone()),
    });

    for node in document.tree.root().descendants().skip(1) {
        if hidden(node) {
            continue;
        }
        let source_id = inventory.source(node.id());
        let Some(mut unit) = unit_for_node(node, source_id, &base_url, &mut context)? else {
            continue;
        };
        unit.parent_source =
            nearest_element_parent(node).map(|parent| inventory.source(parent.id()));
        units.push(unit);
        enforce(
            units.len() as u64,
            limits.semantic_units(),
            ResourceKind::SemanticUnits,
        )?;
    }

    let relationship_count = units
        .iter()
        .map(|unit| unit.relationships.len() as u64)
        .sum();
    let elapsed = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let measurements = MeasurementSet::unavailable_all(UnavailableReason::NotMeasured)
        .with(
            MeasurementKind::LatencyMicros,
            Measurement::Exact {
                value: elapsed,
                source: MeasurementSource::RuntimeCounter,
            },
        )
        .with(
            MeasurementKind::CpuMicros,
            Measurement::Unavailable(UnavailableReason::SourceMissing),
        )
        .with(
            MeasurementKind::PeakMemoryBytes,
            Measurement::Unavailable(UnavailableReason::SourceMissing),
        );
    let diagnostics = ExtractionDiagnostics::new(
        source.bytes.len() as u64,
        inventory.node_count,
        units.len() as u64,
        relationship_count,
        notices.len() as u64,
        measurements,
    );

    Ok(SemanticDocument {
        session: source.session,
        document_url: source.document_url.clone(),
        base_url,
        title: title_text,
        language,
        units,
        notices,
        diagnostics,
    })
}

struct Inventory {
    source_ids: HashMap<NodeId, SourceNodeId>,
    ids: HashMap<String, Vec<NodeId>>,
    labels: HashMap<String, Vec<NodeId>>,
    node_count: u64,
}

impl Inventory {
    fn build(document: &Html, limits: &ExtractionLimits) -> Result<Self, OperationFailure> {
        let mut source_ids = HashMap::new();
        let mut ids: HashMap<String, Vec<NodeId>> = HashMap::new();
        let mut labels: HashMap<String, Vec<NodeId>> = HashMap::new();
        let mut node_count = 0_u64;
        let mut text_bytes = 0_u64;
        for (index, node) in document.tree.root().descendants().enumerate() {
            node_count += 1;
            enforce(node_count, limits.dom_nodes(), ResourceKind::DomNodes)?;
            let depth = node.ancestors().count() as u64;
            enforce(depth, limits.dom_depth(), ResourceKind::DomDepth)?;
            source_ids.insert(node.id(), SourceNodeId::new((index + 1) as u32));
            if let Some(text) = node.value().as_text() {
                text_bytes = text_bytes.saturating_add(text.len() as u64);
                enforce(
                    text_bytes,
                    limits.document_text_bytes(),
                    ResourceKind::DocumentTextBytes,
                )?;
            }
            if let Some(element) = node.value().as_element() {
                enforce(
                    element.attrs.len() as u64,
                    limits.attributes_per_element(),
                    ResourceKind::HtmlAttributes,
                )?;
                if let Some(id) = element.id() {
                    ids.entry(id.to_owned()).or_default().push(node.id());
                }
                if element.name() == "label"
                    && let Some(target) = element.attr("for")
                {
                    labels.entry(target.to_owned()).or_default().push(node.id());
                }
            }
        }
        Ok(Self {
            source_ids,
            ids,
            labels,
            node_count,
        })
    }

    fn source(&self, node: NodeId) -> SourceNodeId {
        self.source_ids[&node]
    }
}

struct Context<'a> {
    inventory: &'a Inventory,
    notices: &'a mut Vec<ExtractionNotice>,
    limits: &'a ExtractionLimits,
}

fn unit_for_node(
    node: NodeRef<'_, Node>,
    source: SourceNodeId,
    base_url: &AbsoluteUrl,
    context: &mut Context<'_>,
) -> Result<Option<ExtractedSemanticUnit>, OperationFailure> {
    let Some(element) = ElementRef::wrap(node) else {
        return Ok(text_unit(node, source));
    };
    let (role, unsupported_explicit) = semantic_role(element);
    if unsupported_explicit {
        push_notice(
            context.notices,
            ExtractionNotice::new(ExtractionNoticeKind::UnsupportedExplicitRole, Some(source)),
            context.limits,
        )?;
    }
    let Some((role, origin)) = role else {
        return Ok(None);
    };
    let allow_content = name_from_content(role);
    let (name, labelled_by) = accessible_name(element, allow_content, source, context)?;
    let description = description(element, source, context)?;
    let value = semantic_value(element, role);
    let state = element_state(element, role);
    let destination = destination(element, role, base_url, source, context)?;
    let affordances = affordances(element, role, &state, &destination);
    let mut relationships = labelled_by;
    relationships.extend(idref_relationships(
        element,
        "aria-describedby",
        RelationshipKind::DescribedBy,
        source,
        context,
    )?);
    relationships.extend(idref_relationships(
        element,
        "aria-controls",
        RelationshipKind::Controls,
        source,
        context,
    )?);
    add_structural_relationships(element, role, context.inventory, &mut relationships);
    relationships.sort_unstable();
    relationships.dedup();
    enforce(
        relationships.len() as u64,
        context.limits.relationships_per_unit(),
        ResourceKind::SemanticRelationships,
    )?;

    Ok(Some(ExtractedSemanticUnit {
        source,
        parent_source: None,
        author_id: author_identity(element, context.inventory),
        role,
        role_origin: origin,
        provenance: Provenance::UntrustedWebContent,
        name,
        description,
        value,
        state,
        relationships,
        affordances,
        destination,
    }))
}

fn text_unit(node: NodeRef<'_, Node>, source: SourceNodeId) -> Option<ExtractedSemanticUnit> {
    let text = node.value().as_text()?;
    let normalized = normalize(text);
    if normalized.is_empty() {
        return None;
    }
    let parent = nearest_element_parent(node)?;
    if !matches!(
        parent.value().as_element()?.name(),
        "body" | "div" | "span" | "main" | "article" | "section"
    ) {
        return None;
    }
    let name = bounded_optional::<MAX_NAME_BYTES>(&normalized).map(Property::Known)?;
    Some(ExtractedSemanticUnit {
        source,
        parent_source: None,
        author_id: Property::NotApplicable,
        role: SemanticRole::Text,
        role_origin: RoleOrigin::NativeHtml,
        provenance: Provenance::UntrustedWebContent,
        name,
        description: Property::NotApplicable,
        value: SemanticValue::Absent,
        state: ElementState::new(),
        relationships: Vec::new(),
        affordances: ActionAffordances::default(),
        destination: Property::NotApplicable,
    })
}

fn author_identity(
    element: ElementRef<'_>,
    inventory: &Inventory,
) -> Property<BoundedText<MAX_AUTHOR_ID_BYTES>> {
    let Some(value) = element.attr("id") else {
        return Property::NotApplicable;
    };
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        return Property::Unknown(PropertyUnknownReason::Unsupported);
    }
    if inventory
        .ids
        .get(value)
        .is_none_or(|matches| matches.len() != 1)
    {
        return Property::Unknown(PropertyUnknownReason::Ambiguous);
    }
    BoundedText::new(value, "author_id")
        .map(Property::Known)
        .unwrap_or(Property::Unknown(PropertyUnknownReason::Unsupported))
}

fn accessible_name(
    element: ElementRef<'_>,
    allow_content: bool,
    source: SourceNodeId,
    context: &mut Context<'_>,
) -> Result<
    (
        Property<BoundedText<MAX_NAME_BYTES>>,
        Vec<ExtractedRelationship>,
    ),
    OperationFailure,
> {
    let mut visited = HashSet::new();
    visited.insert(element.id());
    if let Some(idrefs) = element.attr("aria-labelledby") {
        let mut parts = Vec::new();
        let mut relationships = Vec::new();
        let mut ambiguous = false;
        let idrefs = idrefs.split_ascii_whitespace().collect::<Vec<_>>();
        enforce(
            idrefs.len() as u64,
            context.limits.relationships_per_unit(),
            ResourceKind::SemanticRelationships,
        )?;
        for id in idrefs {
            match context.inventory.ids.get(id) {
                Some(nodes) if nodes.len() == 1 => {
                    let target = nodes[0];
                    if !visited.insert(target) {
                        push_notice(
                            context.notices,
                            ExtractionNotice::new(
                                ExtractionNoticeKind::CyclicNameReference,
                                Some(source),
                            ),
                            context.limits,
                        )?;
                        ambiguous = true;
                        continue;
                    }
                    if let Some(target_node) = element.tree().get(target) {
                        parts.push(visible_text(target_node, true));
                        relationships.push(ExtractedRelationship::new(
                            RelationshipKind::LabelledBy,
                            context.inventory.source(target),
                        ));
                    }
                }
                Some(_) => {
                    push_notice(
                        context.notices,
                        ExtractionNotice::new(ExtractionNoticeKind::DuplicateHtmlId, Some(source)),
                        context.limits,
                    )?;
                    ambiguous = true;
                }
                None => {
                    push_notice(
                        context.notices,
                        ExtractionNotice::new(
                            ExtractionNoticeKind::BrokenIdReference,
                            Some(source),
                        ),
                        context.limits,
                    )?;
                    ambiguous = true;
                }
            }
        }
        let text = normalize(&parts.join(" "));
        if !text.is_empty() && !ambiguous {
            return Ok((known_name(&text), relationships));
        }
        if ambiguous {
            return Ok((
                Property::Unknown(PropertyUnknownReason::Ambiguous),
                relationships,
            ));
        }
    }
    if let Some(label) = element
        .attr("aria-label")
        .map(normalize)
        .filter(|value| !value.is_empty())
    {
        return Ok((known_name(&label), Vec::new()));
    }
    if is_labelable(element) {
        let mut label_nodes = Vec::new();
        if let Some(id) = element.value().id()
            && let Some(nodes) = context.inventory.labels.get(id)
        {
            label_nodes.extend(nodes.iter().copied());
        }
        if let Some(label) = element.ancestors().find(|ancestor| {
            ancestor
                .value()
                .as_element()
                .is_some_and(|value| value.name() == "label")
        }) {
            label_nodes.push(label.id());
        }
        label_nodes.sort_unstable();
        label_nodes.dedup();
        if label_nodes.len() == 1 {
            let label = label_nodes[0];
            if let Some(label_node) = element.tree().get(label) {
                let text = visible_text(label_node, true);
                if !text.is_empty() {
                    return Ok((
                        known_name(&text),
                        vec![ExtractedRelationship::new(
                            RelationshipKind::LabelledBy,
                            context.inventory.source(label),
                        )],
                    ));
                }
            }
        } else if label_nodes.len() > 1 {
            return Ok((
                Property::Unknown(PropertyUnknownReason::Ambiguous),
                Vec::new(),
            ));
        }
    }
    if element.value().name() == "input" {
        let input_type = element.attr("type").unwrap_or("text");
        if input_type.eq_ignore_ascii_case("image")
            && let Some(alt) = element
                .attr("alt")
                .map(normalize)
                .filter(|value| !value.is_empty())
        {
            return Ok((known_name(&alt), Vec::new()));
        }
        if matches!(
            input_type.to_ascii_lowercase().as_str(),
            "button" | "submit" | "reset"
        ) && let Some(value) = element
            .attr("value")
            .map(normalize)
            .filter(|value| !value.is_empty())
        {
            return Ok((known_name(&value), Vec::new()));
        }
    }
    if allow_content {
        let text = visible_text(*element, false);
        if !text.is_empty() {
            return Ok((known_name(&text), Vec::new()));
        }
    }
    if let Some(title) = element
        .attr("title")
        .map(normalize)
        .filter(|value| !value.is_empty())
    {
        return Ok((known_name(&title), Vec::new()));
    }
    if matches!(input_role(element), Some(SemanticRole::Textbox))
        && let Some(placeholder) = element
            .attr("placeholder")
            .map(normalize)
            .filter(|value| !value.is_empty())
    {
        return Ok((known_name(&placeholder), Vec::new()));
    }
    Ok((
        Property::Unknown(PropertyUnknownReason::NotExposed),
        Vec::new(),
    ))
}

fn description(
    element: ElementRef<'_>,
    source: SourceNodeId,
    context: &mut Context<'_>,
) -> Result<Property<BoundedText<MAX_DESCRIPTION_BYTES>>, OperationFailure> {
    let relationships = resolve_idrefs(element, "aria-describedby", source, context)?;
    if relationships.ambiguous {
        return Ok(Property::Unknown(PropertyUnknownReason::Ambiguous));
    }
    let text = normalize(&relationships.text.join(" "));
    Ok(bounded_optional::<MAX_DESCRIPTION_BYTES>(&text)
        .map_or(Property::NotApplicable, Property::Known))
}

struct ResolvedIdrefs {
    nodes: Vec<NodeId>,
    text: Vec<String>,
    ambiguous: bool,
}

fn resolve_idrefs(
    element: ElementRef<'_>,
    attribute: &str,
    source: SourceNodeId,
    context: &mut Context<'_>,
) -> Result<ResolvedIdrefs, OperationFailure> {
    let mut result = ResolvedIdrefs {
        nodes: Vec::new(),
        text: Vec::new(),
        ambiguous: false,
    };
    let Some(value) = element.attr(attribute) else {
        return Ok(result);
    };
    let idrefs = value.split_ascii_whitespace().collect::<Vec<_>>();
    enforce(
        idrefs.len() as u64,
        context.limits.relationships_per_unit(),
        ResourceKind::SemanticRelationships,
    )?;
    for id in idrefs {
        match context.inventory.ids.get(id) {
            Some(nodes) if nodes.len() == 1 => {
                result.nodes.push(nodes[0]);
                if let Some(node) = element.tree().get(nodes[0]) {
                    result.text.push(visible_text(node, true));
                }
            }
            Some(_) => {
                result.ambiguous = true;
                push_notice(
                    context.notices,
                    ExtractionNotice::new(ExtractionNoticeKind::DuplicateHtmlId, Some(source)),
                    context.limits,
                )?;
            }
            None => {
                result.ambiguous = true;
                push_notice(
                    context.notices,
                    ExtractionNotice::new(ExtractionNoticeKind::BrokenIdReference, Some(source)),
                    context.limits,
                )?;
            }
        }
    }
    Ok(result)
}

fn idref_relationships(
    element: ElementRef<'_>,
    attribute: &str,
    kind: RelationshipKind,
    source: SourceNodeId,
    context: &mut Context<'_>,
) -> Result<Vec<ExtractedRelationship>, OperationFailure> {
    Ok(resolve_idrefs(element, attribute, source, context)?
        .nodes
        .into_iter()
        .map(|node| ExtractedRelationship::new(kind, context.inventory.source(node)))
        .collect())
}

fn add_structural_relationships(
    element: ElementRef<'_>,
    role: SemanticRole,
    inventory: &Inventory,
    relationships: &mut Vec<ExtractedRelationship>,
) {
    let relation = match role {
        SemanticRole::Option => Some((RelationshipKind::OptionOf, &["select"][..])),
        SemanticRole::Row => Some((RelationshipKind::RowOf, &["table"][..])),
        SemanticRole::Cell => Some((RelationshipKind::CellOf, &["tr"][..])),
        SemanticRole::ListItem => Some((RelationshipKind::ListItemOf, &["ul", "ol", "menu"][..])),
        SemanticRole::Textbox
        | SemanticRole::Checkbox
        | SemanticRole::Radio
        | SemanticRole::Select
        | SemanticRole::Button => {
            if let Some(form) = associated_form(element, inventory) {
                relationships.push(ExtractedRelationship::new(
                    RelationshipKind::OwnedBy,
                    inventory.source(form.id()),
                ));
            }
            None
        }
        _ => None,
    };
    if let Some((kind, tags)) = relation
        && let Some(parent) = element.ancestors().find(|ancestor| {
            ancestor
                .value()
                .as_element()
                .is_some_and(|value| tags.contains(&value.name()))
        })
    {
        relationships.push(ExtractedRelationship::new(
            kind,
            inventory.source(parent.id()),
        ));
    }
}

fn semantic_value(element: ElementRef<'_>, role: SemanticRole) -> SemanticValue {
    let value = match role {
        SemanticRole::Textbox if element.value().name() == "textarea" => {
            visible_text(*element, true)
        }
        SemanticRole::Textbox
            if element
                .attr("type")
                .is_some_and(|value| value.eq_ignore_ascii_case("password")) =>
        {
            return SemanticValue::Redacted;
        }
        SemanticRole::Textbox | SemanticRole::Checkbox | SemanticRole::Radio => {
            element.attr("value").map_or_else(String::new, normalize)
        }
        SemanticRole::Option => element
            .attr("value")
            .map_or_else(|| visible_text(*element, true), normalize),
        SemanticRole::Select => element
            .descendent_elements()
            .filter(|child| {
                child.value().name() == "option" && option_selected(*child) == Property::Known(true)
            })
            .map(|child| visible_text(*child, true))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    };
    if value.is_empty() {
        SemanticValue::Absent
    } else {
        SemanticValue::text(truncate::<4096>(&value)).expect("value was bounded")
    }
}

fn destination(
    element: ElementRef<'_>,
    role: SemanticRole,
    base_url: &AbsoluteUrl,
    source: SourceNodeId,
    context: &mut Context<'_>,
) -> Result<Property<AbsoluteUrl>, OperationFailure> {
    match role {
        SemanticRole::Link => element.attr("href").map_or_else(
            || Ok(Property::Unknown(PropertyUnknownReason::NotExposed)),
            |value| resolve_url(base_url, value, source, context),
        ),
        SemanticRole::Form => element.attr("action").map_or_else(
            || Ok(Property::Known(base_url.clone())),
            |value| resolve_url(base_url, value, source, context),
        ),
        SemanticRole::Button if is_submit_button(element) => {
            if let Some(value) = element.attr("formaction") {
                return resolve_url(base_url, value, source, context);
            }
            let Some(form) = associated_form(element, context.inventory) else {
                return Ok(Property::NotApplicable);
            };
            form.attr("action").map_or_else(
                || Ok(Property::Known(base_url.clone())),
                |value| resolve_url(base_url, value, source, context),
            )
        }
        _ => Ok(Property::NotApplicable),
    }
}

fn resolve_url(
    base_url: &AbsoluteUrl,
    value: &str,
    source: SourceNodeId,
    context: &mut Context<'_>,
) -> Result<Property<AbsoluteUrl>, OperationFailure> {
    let base = Url::parse(base_url.as_str()).expect("validated document URL parsed earlier");
    let Ok(resolved) = base.join(value) else {
        push_notice(
            context.notices,
            ExtractionNotice::new(ExtractionNoticeKind::InvalidUrl, Some(source)),
            context.limits,
        )?;
        return Ok(Property::Unknown(PropertyUnknownReason::Ambiguous));
    };
    if !matches!(resolved.scheme(), "http" | "https") {
        push_notice(
            context.notices,
            ExtractionNotice::new(ExtractionNoticeKind::UnsupportedUrlScheme, Some(source)),
            context.limits,
        )?;
        return Ok(Property::Unknown(PropertyUnknownReason::Unsupported));
    }
    AbsoluteUrl::new(resolved.to_string())
        .map(Property::Known)
        .map_err(OperationFailure::InvalidInput)
}

fn affordances(
    element: ElementRef<'_>,
    role: SemanticRole,
    state: &ElementState,
    destination: &Property<AbsoluteUrl>,
) -> ActionAffordances {
    if state.disabled() == &Property::Known(true) {
        return ActionAffordances::default();
    }
    let mut actions = ActionAffordances::default();
    match role {
        SemanticRole::Link if matches!(destination, Property::Known(_)) => {
            actions = actions.with(ActionKind::Follow)
        }
        SemanticRole::Textbox => actions = actions.with(ActionKind::Fill),
        SemanticRole::Checkbox => {
            actions = actions.with(if state.checked() == &Property::Known(true) {
                ActionKind::Uncheck
            } else {
                ActionKind::Check
            })
        }
        SemanticRole::Radio => actions = actions.with(ActionKind::Check),
        SemanticRole::Select | SemanticRole::Option => actions = actions.with(ActionKind::Select),
        SemanticRole::Form if matches!(destination, Property::Known(_)) => {
            actions = actions.with(ActionKind::Submit);
        }
        SemanticRole::Button => {
            actions = actions.with(ActionKind::Press);
            if is_submit_button(element) && matches!(destination, Property::Known(_)) {
                actions = actions.with(ActionKind::Submit);
            }
        }
        _ => {}
    }
    actions
}

fn is_submit_button(element: ElementRef<'_>) -> bool {
    match element.value().name() {
        "button" => !element
            .attr("type")
            .is_some_and(|value| matches!(value.to_ascii_lowercase().as_str(), "button" | "reset")),
        "input" => element
            .attr("type")
            .is_some_and(|value| value.eq_ignore_ascii_case("submit")),
        _ => false,
    }
}

fn associated_form<'a>(element: ElementRef<'a>, inventory: &Inventory) -> Option<ElementRef<'a>> {
    if let Some(id) = element.attr("form") {
        let targets = inventory.ids.get(id)?;
        if targets.len() != 1 {
            return None;
        }
        let form = ElementRef::wrap(element.tree().get(targets[0])?)?;
        return (form.value().name() == "form").then_some(form);
    }
    element
        .ancestors()
        .filter_map(ElementRef::wrap)
        .find(|ancestor| ancestor.value().name() == "form")
}

fn resolve_base_url(
    document: &Html,
    document_url: &Url,
    inventory: &Inventory,
    notices: &mut Vec<ExtractionNotice>,
    limits: &ExtractionLimits,
) -> Result<AbsoluteUrl, OperationFailure> {
    let mut resolved = document_url.clone();
    if let Some(base) = first_element(document, "base")
        && let Some(href) = base.attr("href")
    {
        match document_url.join(href) {
            Ok(candidate) if matches!(candidate.scheme(), "http" | "https") => resolved = candidate,
            Ok(_) => push_notice(
                notices,
                ExtractionNotice::new(
                    ExtractionNoticeKind::UnsupportedUrlScheme,
                    Some(inventory.source(base.id())),
                ),
                limits,
            )?,
            Err(_) => push_notice(
                notices,
                ExtractionNotice::new(
                    ExtractionNoticeKind::InvalidUrl,
                    Some(inventory.source(base.id())),
                ),
                limits,
            )?,
        }
    }
    AbsoluteUrl::new(resolved.to_string()).map_err(OperationFailure::InvalidInput)
}

fn enforce(
    actual: u64,
    limit: std::num::NonZeroU64,
    resource: ResourceKind,
) -> Result<(), OperationFailure> {
    if actual > limit.get() {
        Err(OperationFailure::ResourceLimit {
            resource,
            configured_limit: limit,
        })
    } else {
        Ok(())
    }
}

fn push_notice(
    notices: &mut Vec<ExtractionNotice>,
    notice: ExtractionNotice,
    limits: &ExtractionLimits,
) -> Result<(), OperationFailure> {
    notices.push(notice);
    enforce(
        notices.len() as u64,
        limits.notices(),
        ResourceKind::ExtractionNotices,
    )
}
