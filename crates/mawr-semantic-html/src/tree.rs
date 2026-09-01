use ego_tree::NodeRef;

use crate::dom::{ElementRef, Html, Node};
use crate::normalize::normalize;

pub(crate) fn first_element<'a>(document: &'a Html, tag: &str) -> Option<ElementRef<'a>> {
    document
        .tree
        .root()
        .descendants()
        .filter_map(ElementRef::wrap)
        .find(|element| element.value().name() == tag)
}

pub(crate) fn hidden(node: NodeRef<'_, Node>) -> bool {
    std::iter::once(node)
        .chain(node.ancestors())
        .any(|ancestor| {
            let Some(element) = ancestor.value().as_element() else {
                return false;
            };
            if element.attr("hidden").is_some()
                || element.attr("aria-hidden") == Some("true")
                || element.name() == "template"
                || element.name() == "script"
                || element.name() == "style"
            {
                return true;
            }
            if element.name() == "input"
                && element
                    .attr("type")
                    .is_some_and(|value| value.eq_ignore_ascii_case("hidden"))
            {
                return true;
            }
            let style = element
                .attr("style")
                .unwrap_or_default()
                .to_ascii_lowercase()
                .replace(' ', "");
            style.split(';').any(|declaration| {
                matches!(
                    declaration,
                    "display:none"
                        | "visibility:hidden"
                        | "visibility:collapse"
                        | "content-visibility:hidden"
                )
            })
        })
}

pub(crate) fn visible_text(node: NodeRef<'_, Node>, include_hidden_root: bool) -> String {
    let mut parts = Vec::new();
    for descendant in node.descendants() {
        if descendant != node && hidden(descendant) && !include_hidden_root {
            continue;
        }
        if let Some(text) = descendant.value().as_text() {
            parts.push(text.to_owned());
        }
    }
    normalize(&parts.join(" "))
}

pub(crate) fn nearest_element_parent(node: NodeRef<'_, Node>) -> Option<NodeRef<'_, Node>> {
    node.ancestors()
        .find(|ancestor| ancestor.value().is_element())
}
