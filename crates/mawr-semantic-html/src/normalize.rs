use mawr_core::{BoundedText, Property};

use crate::model::MAX_NAME_BYTES;

pub(crate) fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn truncate<const MAX: usize>(value: &str) -> String {
    if value.len() <= MAX {
        return value.to_owned();
    }
    let mut end = MAX;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

pub(crate) fn bounded_optional<const MAX: usize>(value: &str) -> Option<BoundedText<MAX>> {
    let value = normalize(value);
    (!value.is_empty()).then(|| {
        BoundedText::new(truncate::<MAX>(&value), "semantic_text").expect("text was bounded")
    })
}

pub(crate) fn known_name(value: &str) -> Property<BoundedText<MAX_NAME_BYTES>> {
    Property::Known(
        BoundedText::new(truncate::<MAX_NAME_BYTES>(value), "semantic_name")
            .expect("name was bounded"),
    )
}
