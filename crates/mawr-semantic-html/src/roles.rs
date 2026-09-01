use mawr_core::SemanticRole;

use crate::dom::ElementRef;
use crate::model::RoleOrigin;

pub(crate) fn semantic_role(element: ElementRef<'_>) -> (Option<(SemanticRole, RoleOrigin)>, bool) {
    let mut unsupported_explicit = false;
    if let Some(explicit) = element
        .attr("role")
        .and_then(|value| value.split_ascii_whitespace().next())
    {
        if matches!(explicit, "none" | "presentation") {
            return (None, false);
        }
        if let Some(role) = aria_role(explicit) {
            return (Some((role, RoleOrigin::ExplicitAria)), false);
        }
        unsupported_explicit = true;
    }
    let role = match element.value().name() {
        "main" | "nav" | "aside" | "header" | "footer" => Some(SemanticRole::Region),
        "section" | "article" if has_author_name(element) => Some(SemanticRole::Region),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Some(SemanticRole::Heading),
        "p" | "pre" | "blockquote" | "address" | "dt" | "dd" => Some(SemanticRole::Text),
        "a" if element.attr("href").is_some() => Some(SemanticRole::Link),
        "form" => Some(SemanticRole::Form),
        "textarea" => Some(SemanticRole::Textbox),
        "select" => Some(SemanticRole::Select),
        "option" => Some(SemanticRole::Option),
        "button" => Some(SemanticRole::Button),
        "table" => Some(SemanticRole::Table),
        "tr" => Some(SemanticRole::Row),
        "th" | "td" => Some(SemanticRole::Cell),
        "ul" | "ol" | "menu" => Some(SemanticRole::List),
        "li" => Some(SemanticRole::ListItem),
        "input" => input_role(element),
        _ => None,
    };
    (
        role.map(|role| (role, RoleOrigin::NativeHtml)),
        unsupported_explicit,
    )
}

fn aria_role(value: &str) -> Option<SemanticRole> {
    match value {
        "region" | "main" | "navigation" | "complementary" | "banner" | "contentinfo" => {
            Some(SemanticRole::Region)
        }
        "heading" => Some(SemanticRole::Heading),
        "link" => Some(SemanticRole::Link),
        "form" => Some(SemanticRole::Form),
        "textbox" | "searchbox" => Some(SemanticRole::Textbox),
        "checkbox" | "switch" => Some(SemanticRole::Checkbox),
        "radio" => Some(SemanticRole::Radio),
        "listbox" | "combobox" => Some(SemanticRole::Select),
        "option" => Some(SemanticRole::Option),
        "button" => Some(SemanticRole::Button),
        "table" | "grid" => Some(SemanticRole::Table),
        "row" => Some(SemanticRole::Row),
        "cell" | "gridcell" | "columnheader" | "rowheader" => Some(SemanticRole::Cell),
        "list" => Some(SemanticRole::List),
        "listitem" => Some(SemanticRole::ListItem),
        "alert" => Some(SemanticRole::Alert),
        _ => None,
    }
}

pub(crate) fn input_role(element: ElementRef<'_>) -> Option<SemanticRole> {
    match element
        .attr("type")
        .unwrap_or("text")
        .to_ascii_lowercase()
        .as_str()
    {
        "hidden" => None,
        "checkbox" => Some(SemanticRole::Checkbox),
        "radio" => Some(SemanticRole::Radio),
        "button" | "submit" | "reset" | "image" => Some(SemanticRole::Button),
        _ => Some(SemanticRole::Textbox),
    }
}

pub(crate) fn is_labelable(element: ElementRef<'_>) -> bool {
    matches!(
        element.value().name(),
        "button" | "input" | "select" | "textarea"
    )
}

pub(crate) fn name_from_content(role: SemanticRole) -> bool {
    matches!(
        role,
        SemanticRole::Region
            | SemanticRole::Heading
            | SemanticRole::Text
            | SemanticRole::Link
            | SemanticRole::Option
            | SemanticRole::Button
            | SemanticRole::Cell
            | SemanticRole::ListItem
            | SemanticRole::Alert
            | SemanticRole::Table
    )
}

fn has_author_name(element: ElementRef<'_>) -> bool {
    ["aria-label", "aria-labelledby", "title"]
        .into_iter()
        .any(|attribute| {
            element
                .attr(attribute)
                .is_some_and(|value| !value.trim().is_empty())
        })
}
