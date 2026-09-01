use mawr_core::{ElementState, Property, PropertyUnknownReason, SemanticRole};

use crate::dom::ElementRef;

pub(crate) fn element_state(element: ElementRef<'_>, role: SemanticRole) -> ElementState {
    let control = matches!(
        role,
        SemanticRole::Textbox
            | SemanticRole::Checkbox
            | SemanticRole::Radio
            | SemanticRole::Select
            | SemanticRole::Option
            | SemanticRole::Button
    );
    let mut state = ElementState::new();
    if control {
        state = state.with_disabled(Property::Known(is_disabled(element)));
    }
    if matches!(role, SemanticRole::Checkbox | SemanticRole::Radio) {
        state = state.with_checked(boolean_state(element, "checked", "aria-checked"));
    }
    if role == SemanticRole::Option {
        state = state.with_selected(option_selected(element));
    }
    if element.attr("aria-expanded").is_some() {
        state = state.with_expanded(aria_boolean(element.attr("aria-expanded")));
    }
    if matches!(
        role,
        SemanticRole::Textbox | SemanticRole::Checkbox | SemanticRole::Radio | SemanticRole::Select
    ) {
        state = state.with_required(boolean_state(element, "required", "aria-required"));
        state = state.with_invalid(match element.attr("aria-invalid") {
            None | Some("false") => Property::Known(false),
            Some("true" | "grammar" | "spelling") => Property::Known(true),
            Some(_) => Property::Unknown(PropertyUnknownReason::Ambiguous),
        });
    }
    state
}

fn boolean_state(element: ElementRef<'_>, native: &str, aria: &str) -> Property<bool> {
    element
        .attr(aria)
        .map_or(Property::Known(element.attr(native).is_some()), |value| {
            aria_boolean(Some(value))
        })
}

fn aria_boolean(value: Option<&str>) -> Property<bool> {
    match value {
        Some("true") => Property::Known(true),
        Some("false") => Property::Known(false),
        _ => Property::Unknown(PropertyUnknownReason::Ambiguous),
    }
}

fn is_disabled(element: ElementRef<'_>) -> bool {
    if element.attr("disabled").is_some()
        || element
            .attr("aria-disabled")
            .is_some_and(|value| value == "true")
    {
        return true;
    }
    for ancestor in element.ancestors().filter_map(ElementRef::wrap) {
        if ancestor.value().name() != "fieldset" || ancestor.attr("disabled").is_none() {
            continue;
        }
        let first_legend = ancestor
            .child_elements()
            .find(|child| child.value().name() == "legend");
        if first_legend.is_some_and(|legend| {
            element
                .ancestors()
                .any(|candidate| candidate.id() == legend.id())
        }) {
            continue;
        }
        return true;
    }
    false
}

pub(crate) fn option_selected(element: ElementRef<'_>) -> Property<bool> {
    if element.attr("aria-selected").is_some() {
        return aria_boolean(element.attr("aria-selected"));
    }
    if element.attr("selected").is_some() {
        return Property::Known(true);
    }
    let Some(select) = element
        .ancestors()
        .filter_map(ElementRef::wrap)
        .find(|ancestor| ancestor.value().name() == "select")
    else {
        return Property::Known(false);
    };
    let options = select
        .descendent_elements()
        .filter(|candidate| candidate.value().name() == "option")
        .collect::<Vec<_>>();
    if options
        .iter()
        .any(|candidate| candidate.attr("selected").is_some())
    {
        return Property::Known(false);
    }
    Property::Known(
        options
            .into_iter()
            .find(|candidate| !is_disabled(*candidate))
            .is_some_and(|candidate| candidate.id() == element.id()),
    )
}
