use std::collections::BTreeSet;

use mawr_core::{
    AbsoluteUrl, Action, ActionKind, ActionRequest, BoundedU64, Capability, CapabilityReport,
    CapabilityStatus, CollectionLimit, ElementRef, EngineIdentity, EngineKind, FailureClass,
    FullObservationReason, Measurement, MeasurementKind, MeasurementSet, MeasurementSource,
    Observation, ObservationBasis, PageId, PageIdentity, Provenance, SemanticRole, SemanticUnit,
    SessionId, StateId, UnsupportedReason,
};

fn engine() -> EngineIdentity {
    EngineIdentity::new("native-static", "0.0.0", EngineKind::NativeStatic).unwrap()
}

#[test]
fn vocabularies_are_exhaustive_unique_and_stably_ordered() {
    assert_eq!(
        Capability::ALL.into_iter().collect::<BTreeSet<_>>().len(),
        Capability::COUNT
    );
    assert_eq!(
        SemanticRole::ALL.into_iter().collect::<BTreeSet<_>>().len(),
        SemanticRole::COUNT
    );
    assert_eq!(
        ActionKind::ALL.into_iter().collect::<BTreeSet<_>>().len(),
        ActionKind::COUNT
    );
    assert_eq!(
        FailureClass::ALL.into_iter().collect::<BTreeSet<_>>().len(),
        FailureClass::COUNT
    );
    assert_eq!(MeasurementKind::ALL.len(), MeasurementKind::COUNT);
}

#[test]
fn same_inputs_produce_equal_canonical_observations() {
    let session = SessionId::new(11).unwrap();
    let state = StateId::new(session, 3).unwrap();
    let page = PageIdentity::new(
        PageId::new(session, 2).unwrap(),
        AbsoluteUrl::new("https://example.test/form").unwrap(),
    );
    let engine = engine();
    let capabilities =
        CapabilityReport::unsupported_all(engine.clone(), UnsupportedReason::EngineLimitation)
            .with(Capability::HtmlParsing, CapabilityStatus::Supported);
    let first = SemanticUnit::new(
        ElementRef::new(session, 2).unwrap(),
        SemanticRole::Button,
        Provenance::UntrustedWebContent,
    );
    let second = SemanticUnit::new(
        ElementRef::new(session, 1).unwrap(),
        SemanticRole::Heading,
        Provenance::UntrustedWebContent,
    );
    let metrics = MeasurementSet::default().with(
        MeasurementKind::LatencyMicros,
        Measurement::Exact {
            value: 12,
            source: MeasurementSource::RuntimeCounter,
        },
    );

    let left = Observation::new(
        state,
        page.clone(),
        engine.clone(),
        capabilities.clone(),
        ObservationBasis::Full(FullObservationReason::Initial),
        CollectionLimit::new(100, "unit_limit").unwrap(),
    )
    .unwrap()
    .with_unit(first.clone())
    .unwrap()
    .with_unit(second.clone())
    .unwrap()
    .with_measurements(metrics.clone());
    let right = Observation::new(
        state,
        page,
        engine,
        capabilities,
        ObservationBasis::Full(FullObservationReason::Initial),
        CollectionLimit::new(100, "unit_limit").unwrap(),
    )
    .unwrap()
    .with_unit(second)
    .unwrap()
    .with_unit(first)
    .unwrap()
    .with_measurements(metrics);

    assert_eq!(left, right);
}

#[test]
fn state_scoping_prevents_cross_session_actions_and_observations() {
    let first = SessionId::new(1).unwrap();
    let second = SessionId::new(2).unwrap();
    let expected = StateId::new(first, 1).unwrap();
    let foreign = ElementRef::new(second, 1).unwrap();
    assert!(ActionRequest::new(expected, Action::follow(foreign)).is_err());

    let engine = engine();
    let capabilities =
        CapabilityReport::unsupported_all(engine.clone(), UnsupportedReason::NotImplemented);
    assert!(
        Observation::new(
            expected,
            PageIdentity::new(
                PageId::new(second, 1).unwrap(),
                AbsoluteUrl::new("https://example.test").unwrap(),
            ),
            engine,
            capabilities,
            ObservationBasis::Full(FullObservationReason::Initial),
            CollectionLimit::new(100, "unit_limit").unwrap(),
        )
        .is_err()
    );
}

#[test]
fn bounded_numeric_property_holds_for_deterministic_generated_values() {
    type Subject = BoundedU64<100, 10_000>;
    let mut state = 0x4d41_5752_u64;

    for _ in 0..20_000 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let value = state % 20_000;
        assert_eq!(
            Subject::new(value, "generated").is_ok(),
            (100..=10_000).contains(&value)
        );
    }
}
