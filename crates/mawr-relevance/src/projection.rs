use std::fmt::Write as _;

use mawr_core::{Observation, Property, SemanticUnit, SemanticValue};

pub(crate) fn envelope_projection(observation: &Observation) -> String {
    let mut output = String::new();
    let _ = write!(
        output,
        "state={} page={} url={} engine={}@{} basis={:?} changes={:?} summary={} ",
        observation.state().sequence(),
        observation.page().id().sequence(),
        observation.page().url().as_str(),
        observation.engine().name(),
        observation.engine().version(),
        observation.basis(),
        observation.changes(),
        observation.summary().unwrap_or_default(),
    );
    for (capability, status) in observation.capabilities().iter() {
        let _ = write!(output, "{capability:?}={status:?} ");
    }
    output
}

pub(crate) fn unit_projection(unit: &SemanticUnit) -> String {
    let mut output = String::new();
    let _ = write!(
        output,
        "ref={} role={:?} provenance={:?} ",
        unit.reference(),
        unit.role(),
        unit.provenance()
    );
    if let Some(parent) = unit.parent() {
        let _ = write!(output, "parent={parent} ");
    }
    write_property_text(&mut output, "name", unit.name());
    write_property_text(&mut output, "description", unit.description());
    match unit.value() {
        SemanticValue::Absent => output.push_str("value=na "),
        SemanticValue::Text(value) => {
            let _ = write!(output, "value={} ", value.as_str());
        }
        SemanticValue::Redacted => output.push_str("value=redacted "),
        SemanticValue::Unknown(reason) => {
            let _ = write!(output, "value_unknown={} ", reason.as_str());
        }
    }
    let _ = write!(output, "state={:?} ", unit.state());
    for relationship in unit.relationships() {
        let _ = write!(
            output,
            "relation={:?}:{} ",
            relationship.kind(),
            relationship.target()
        );
    }
    for affordance in unit.affordances().iter() {
        let _ = write!(output, "action={affordance:?} ");
    }
    match unit.destination() {
        Property::Known(destination) => {
            let _ = write!(output, "destination={} ", destination.as_str());
        }
        Property::Unknown(reason) => {
            let _ = write!(output, "destination_unknown={reason:?} ");
        }
        Property::NotApplicable => {}
    }
    output
}

fn write_property_text<const MAX: usize>(
    output: &mut String,
    field: &str,
    property: &Property<mawr_core::BoundedText<MAX>>,
) {
    match property {
        Property::Known(value) => {
            let _ = write!(output, "{field}={} ", value.as_str());
        }
        Property::Unknown(reason) => {
            let _ = write!(output, "{field}_unknown={reason:?} ");
        }
        Property::NotApplicable => {}
    }
}
