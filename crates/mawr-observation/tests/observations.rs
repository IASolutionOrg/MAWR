use std::collections::BTreeSet;

use mawr_core::{
    AbsoluteUrl, ActionKind, Capability, CapabilityReport, CapabilityStatus, CollectionLimit,
    EngineFailureKind, EngineIdentity, EngineKind, FailureClass, Measurement, MeasurementKind,
    ObservationBasis, ObservationChanges, ObservationRequest, ObservationTokenBudget,
    OmissionCategory, OperationFailure, Property, RelationshipKind, ResetReason, SemanticRole,
    SessionId, StateId, TransitionCause, UnsupportedReason,
};
use mawr_observation::{FullObservationBuilder, FullObservationConfig};
use mawr_semantic_html::{HtmlDocumentSource, HtmlSemanticExtractor, SemanticDocument};
use mawr_state::{SemanticStateStore, StateStoreConfig};

fn engine() -> EngineIdentity {
    EngineIdentity::new("native-static", "0", EngineKind::NativeStatic).unwrap()
}

fn capabilities(engine: &EngineIdentity) -> CapabilityReport {
    CapabilityReport::unsupported_all(engine.clone(), UnsupportedReason::NotImplemented)
        .with(Capability::HtmlParsing, CapabilityStatus::Supported)
        .with(Capability::SemanticContent, CapabilityStatus::Supported)
}

fn document(session: SessionId, url: &str, html: &str) -> SemanticDocument {
    let url = AbsoluteUrl::new(url).unwrap();
    HtmlSemanticExtractor::default()
        .extract_source(HtmlDocumentSource::new(session, &url, html.as_bytes()))
        .unwrap()
}

fn store(session: SessionId, config: StateStoreConfig) -> SemanticStateStore {
    SemanticStateStore::new(session, engine(), config)
}

fn builder() -> FullObservationBuilder {
    let engine = engine();
    FullObservationBuilder::new(capabilities(&engine), FullObservationConfig::default())
}

#[test]
fn full_observation_preserves_roles_properties_relationships_and_affordances() {
    let session = SessionId::new(21).unwrap();
    let mut store = store(session, StateStoreConfig::default());
    store
        .update(
            document(
                session,
                "https://example.test/catalog",
                include_str!("fixtures/all-semantics.html"),
            ),
            TransitionCause::Refresh,
        )
        .unwrap();

    let built = builder()
        .build(&store, &ObservationRequest::new(session))
        .unwrap();
    let observation = built.observation();
    let roles = observation
        .units()
        .iter()
        .map(|unit| unit.role())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        roles,
        SemanticRole::ALL.into_iter().collect::<BTreeSet<_>>()
    );
    assert_eq!(observation.summary(), Some("Semantic catalog"));
    assert_eq!(
        observation.basis(),
        ObservationBasis::Full(mawr_core::FullObservationReason::Initial)
    );
    assert_eq!(observation.changes(), &ObservationChanges::NotRequested);
    assert_eq!(observation.capabilities(), &capabilities(&engine()));
    assert!(
        OmissionCategory::ALL
            .into_iter()
            .all(|category| observation.omissions().count(category) == 0)
    );

    let link = observation
        .units()
        .iter()
        .find(|unit| unit.role() == SemanticRole::Link)
        .unwrap();
    assert!(matches!(
        link.description(),
        Property::Known(value) if value.as_str() == "Opens documentation"
    ));
    assert!(matches!(
        link.destination(),
        Property::Known(value) if value.as_str() == "https://example.test/docs"
    ));
    assert!(link.affordances().contains(ActionKind::Follow));

    let textbox = observation
        .units()
        .iter()
        .find(|unit| unit.role() == SemanticRole::Textbox)
        .unwrap();
    assert!(matches!(
        textbox.value(),
        mawr_core::SemanticValue::Text(value) if value.as_str() == "Ada"
    ));
    assert_eq!(textbox.state().required(), &Property::Known(true));
    assert!(textbox.affordances().contains(ActionKind::Fill));

    let list_item = observation
        .units()
        .iter()
        .find(|unit| unit.role() == SemanticRole::ListItem)
        .unwrap();
    assert!(list_item.parent().is_some());
    assert!(list_item.relationships().any(|relationship| {
        relationship.kind() == RelationshipKind::ListItemOf
            && Some(relationship.target()) == list_item.parent()
    }));
    assert_eq!(built.diagnostics().unit_count(), observation.units().len());
    assert!(built.diagnostics().relationship_count() > 0);
    assert!(built.diagnostics().unresolved_relationship_count() > 0);
    assert!(built.diagnostics().logical_content_bytes() > 0);
    assert_eq!(
        observation
            .measurements()
            .get(MeasurementKind::LatencyMicros),
        &Measurement::Exact {
            value: built.diagnostics().construction_latency_micros(),
            source: mawr_core::MeasurementSource::RuntimeCounter,
        }
    );
}

#[test]
fn retained_evicted_future_and_navigation_bases_are_classified_explicitly() {
    let session = SessionId::new(22).unwrap();
    let html = "<!doctype html><title>State</title><button id='go'>Go</button>";
    let mut retained = store(session, StateStoreConfig::default());
    let first = retained
        .update(
            document(session, "https://example.test/state", html),
            TransitionCause::Refresh,
        )
        .unwrap()
        .transition()
        .to();
    retained
        .update(
            document(session, "https://example.test/state", html),
            TransitionCause::ExternalChange,
        )
        .unwrap();
    let incremental = builder()
        .build(
            &retained,
            &ObservationRequest::new(session).since_state(first).unwrap(),
        )
        .unwrap();
    assert_eq!(
        incremental.observation().basis(),
        ObservationBasis::Incremental { base: first }
    );
    let ObservationChanges::Computed(changes) = incremental.observation().changes() else {
        panic!("retained same-page base must produce computed changes");
    };
    assert_eq!(changes.base(), first);
    assert_eq!(changes.target(), retained.current().unwrap().id());
    assert_eq!(changes.unit_change_count(), 0);
    assert!(incremental.observation().units().is_empty());
    assert_eq!(
        incremental
            .diagnostics()
            .diff()
            .unwrap()
            .emitted_unit_count(),
        0
    );

    let one_state = StateStoreConfig::default().with_retained_states(1).unwrap();
    let mut evicting = store(session, one_state);
    let evicted = evicting
        .update(
            document(session, "https://example.test/state", html),
            TransitionCause::Refresh,
        )
        .unwrap()
        .transition()
        .to();
    evicting
        .update(
            document(session, "https://example.test/state", html),
            TransitionCause::Refresh,
        )
        .unwrap();
    let reset = builder()
        .build(
            &evicting,
            &ObservationRequest::new(session)
                .since_state(evicted)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        reset.observation().basis(),
        ObservationBasis::Reset {
            requested_base: evicted,
            reason: ResetReason::BaseEvicted,
        }
    );

    let future = StateId::new(session, 100).unwrap();
    let unavailable = builder()
        .build(
            &evicting,
            &ObservationRequest::new(session)
                .since_state(future)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        unavailable.observation().changes(),
        &ObservationChanges::Reset {
            requested_base: future,
            reason: ResetReason::BaseUnavailable,
        }
    );

    let mut navigating = store(session, StateStoreConfig::default());
    let old_page = navigating
        .update(
            document(session, "https://example.test/one", html),
            TransitionCause::Refresh,
        )
        .unwrap()
        .transition()
        .to();
    navigating
        .update(
            document(session, "https://example.test/two", html),
            TransitionCause::Navigation,
        )
        .unwrap();
    let navigation = builder()
        .build(
            &navigating,
            &ObservationRequest::new(session)
                .since_state(old_page)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        navigation.observation().basis(),
        ObservationBasis::Reset {
            requested_base: old_page,
            reason: ResetReason::NavigationBoundary,
        }
    );

    let mut ambiguous = store(session, StateStoreConfig::default());
    let ambiguous_base = ambiguous
        .update(
            document(session, "https://example.test/state", html),
            TransitionCause::Refresh,
        )
        .unwrap()
        .transition()
        .to();
    ambiguous
        .update(
            document(session, "https://example.test/state", html),
            TransitionCause::Reset(ResetReason::AmbiguousIdentity),
        )
        .unwrap();
    let ambiguous_reset = builder()
        .build(
            &ambiguous,
            &ObservationRequest::new(session)
                .since_state(ambiguous_base)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        ambiguous_reset.observation().basis(),
        ObservationBasis::Reset {
            requested_base: ambiguous_base,
            reason: ResetReason::AmbiguousIdentity,
        }
    );
}

#[test]
fn empty_store_session_and_engine_mismatches_fail_explicitly() {
    let session = SessionId::new(23).unwrap();
    let empty = store(session, StateStoreConfig::default());
    assert!(matches!(
        builder().build(&empty, &ObservationRequest::new(session)),
        Err(OperationFailure::EngineFailure {
            kind: EngineFailureKind::StateUnavailable,
            ..
        })
    ));

    let mut populated = store(session, StateStoreConfig::default());
    populated
        .update(
            document(session, "https://example.test", "<!doctype html>"),
            TransitionCause::Refresh,
        )
        .unwrap();
    let foreign = SessionId::new(24).unwrap();
    let failure = builder()
        .build(&populated, &ObservationRequest::new(foreign))
        .unwrap_err();
    assert_eq!(failure.class(), FailureClass::InvalidInput);
    assert!(ObservationRequest::new(session).with_goal("   ").is_err());
    assert!(
        ObservationRequest::new(session)
            .with_goal("x".repeat(4_097))
            .is_err()
    );

    let other_engine = EngineIdentity::new("external", "0", EngineKind::ExternalAdapter).unwrap();
    let mismatched = FullObservationBuilder::new(
        capabilities(&other_engine),
        FullObservationConfig::default(),
    );
    assert_eq!(
        mismatched
            .build(&populated, &ObservationRequest::new(session))
            .unwrap_err()
            .class(),
        FailureClass::InvalidInput
    );
}

#[test]
fn empty_and_error_pages_remain_complete_and_honest() {
    let session = SessionId::new(25).unwrap();
    let mut empty = store(session, StateStoreConfig::default());
    empty
        .update(
            document(session, "https://example.test/empty", "<!doctype html>"),
            TransitionCause::Refresh,
        )
        .unwrap();
    let empty_observation = builder()
        .build(&empty, &ObservationRequest::new(session))
        .unwrap();
    assert_eq!(
        empty_observation.observation().summary(),
        Some("Untitled page")
    );
    assert_eq!(empty_observation.observation().units().len(), 1);
    assert_eq!(
        empty_observation.observation().units()[0].role(),
        SemanticRole::Page
    );

    let mut error = store(session, StateStoreConfig::default());
    error
        .update(
            document(
                session,
                "https://example.test/error",
                "<!doctype html><title>Service error</title><div role='alert'>Unavailable</div>",
            ),
            TransitionCause::Refresh,
        )
        .unwrap();
    let error_observation = builder()
        .build(&error, &ObservationRequest::new(session))
        .unwrap();
    assert_eq!(
        error_observation.observation().summary(),
        Some("Service error")
    );
    assert!(
        error_observation
            .observation()
            .units()
            .iter()
            .any(|unit| unit.role() == SemanticRole::Alert)
    );

    let mut bounded = store(session, StateStoreConfig::default());
    let long_title = format!("<!doctype html><title>{}</title>", "é".repeat(600));
    bounded
        .update(
            document(session, "https://example.test/long-title", &long_title),
            TransitionCause::Refresh,
        )
        .unwrap();
    let bounded_observation = builder()
        .build(&bounded, &ObservationRequest::new(session))
        .unwrap();
    assert!(bounded_observation.observation().summary().unwrap().len() <= 1_024);
}

#[test]
fn large_full_state_is_bounded_and_reports_deferred_selection_inputs() {
    let session = SessionId::new(26).unwrap();
    let mut html = String::from("<!doctype html><title>Large state</title>");
    for index in 0..512 {
        html.push_str(&format!(
            "<button id='button-{index}'>Button {index}</button>"
        ));
    }
    let mut store = store(session, StateStoreConfig::default());
    store
        .update(
            document(session, "https://example.test/large", &html),
            TransitionCause::Refresh,
        )
        .unwrap();

    let limited = FullObservationBuilder::new(
        capabilities(&engine()),
        FullObservationConfig::default()
            .with_unit_limit(CollectionLimit::new(100, "observation_unit_limit").unwrap()),
    );
    assert_eq!(
        limited
            .build(&store, &ObservationRequest::new(session))
            .unwrap_err()
            .class(),
        FailureClass::ResourceLimit
    );

    let request = ObservationRequest::new(session)
        .with_goal("Find the final button")
        .unwrap()
        .with_max_tokens(ObservationTokenBudget::new(1_000, "max_tokens").unwrap());
    let built = builder().build(&store, &request).unwrap();
    assert_eq!(built.observation().units().len(), 513);
    assert!(built.diagnostics().goal_deferred());
    assert!(built.diagnostics().token_budget_deferred());
    assert!(built.diagnostics().logical_content_bytes() > 5_000);
    assert_eq!(built.diagnostics().source_input_bytes(), html.len() as u64);
}

#[test]
fn full_observation_structure_and_order_are_deterministic() {
    let session = SessionId::new(27).unwrap();
    let mut store = store(session, StateStoreConfig::default());
    store
        .update(
            document(
                session,
                "https://example.test/catalog",
                include_str!("fixtures/all-semantics.html"),
            ),
            TransitionCause::Refresh,
        )
        .unwrap();
    let request = ObservationRequest::new(session);
    let first = builder().build(&store, &request).unwrap();
    let second = builder().build(&store, &request).unwrap();
    assert_eq!(first.observation().units(), second.observation().units());
    assert_eq!(
        first.observation().summary(),
        second.observation().summary()
    );
    assert_eq!(first.observation().basis(), second.observation().basis());
    assert_eq!(
        first.observation().changes(),
        second.observation().changes()
    );
    assert_eq!(
        first.observation().omissions(),
        second.observation().omissions()
    );
    assert!(
        first
            .observation()
            .units()
            .windows(2)
            .all(|pair| pair[0].reference() < pair[1].reference())
    );
}
