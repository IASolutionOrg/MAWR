use std::collections::BTreeSet;

use mawr_core::{
    AbsoluteUrl, Capability, CapabilityReport, CapabilityStatus, CollectionLimit, ElementRef,
    EngineIdentity, EngineKind, ObservationBasis, ObservationChanges, ObservationRequest, Property,
    ResetReason, SessionId, TransitionCause, UnsupportedReason,
};
use mawr_observation::{FullObservationBuilder, FullObservationConfig, SemanticSnapshot};
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

fn builder() -> FullObservationBuilder {
    let engine = engine();
    FullObservationBuilder::new(capabilities(&engine), FullObservationConfig::default())
}

fn store(session: SessionId, config: StateStoreConfig) -> SemanticStateStore {
    SemanticStateStore::new(session, engine(), config)
}

fn document(session: SessionId, html: &str) -> SemanticDocument {
    let url = AbsoluteUrl::new("https://example.test/state").unwrap();
    HtmlSemanticExtractor::default()
        .extract_source(HtmlDocumentSource::new(session, &url, html.as_bytes()))
        .unwrap()
}

fn reference(store: &SemanticStateStore, id: &str) -> ElementRef {
    store
        .current()
        .unwrap()
        .units()
        .iter()
        .find(|unit| {
            matches!(unit.semantic().author_id(), Property::Known(value) if value.as_str() == id)
        })
        .unwrap_or_else(|| panic!("missing fixture id {id}"))
        .reference()
}

fn boilerplate() -> String {
    (0..24)
        .map(|index| format!("<p id='stable-{index}'>Stable boilerplate {index}</p>"))
        .collect()
}

#[test]
fn semantic_properties_reference_changes_and_order_reconstruct_exactly() {
    let session = SessionId::new(31).unwrap();
    let mut store = store(session, StateStoreConfig::default());
    let base_html = format!(
        "<!doctype html><title>Before</title><section id='parent-a' aria-label='Old region'>
         <button id='expander' aria-expanded='false'>Expand</button>
         <p id='help'>Old help</p>
         <input id='text' aria-label='Old name' aria-describedby='help' value='before' required>
         <input id='check' type='checkbox'>
         <select id='select'><option id='one' selected>One</option><option id='two'>Two</option></select>
         <a id='link' href='/old'>Link</a><button id='role'>Role</button>
         <button id='moved'>Move me</button><div id='removed'>Removed</div>{}</section>
         <section id='parent-b' aria-label='Second region'></section>",
        boilerplate()
    );
    let base = store
        .update(document(session, &base_html), TransitionCause::Refresh)
        .unwrap()
        .transition()
        .to();
    let base_observation = builder()
        .build(&store, &ObservationRequest::new(session))
        .unwrap()
        .into_observation();
    let old_role = reference(&store, "role");

    let target_html = format!(
        "<!doctype html><title>After</title><section id='parent-a' aria-label='New region'>
         <a id='link' href='/new'>Link</a>
         <input id='check' type='checkbox' checked disabled required aria-invalid='true'>
         <button id='expander' aria-expanded='true'>Expand</button>
         <p id='help'>New help</p><p id='help-two'>Additional help</p>
         <input id='text' aria-label='New name' aria-describedby='help-two' value='after' disabled aria-invalid='true'>
         <select id='select'><option id='one'>One</option><option id='two' selected>Two</option></select>
         <h2 id='role'>Role</h2><div id='added' role='alert'>Added alert</div>{}</section>
         <section id='parent-b' aria-label='Second region'><button id='moved'>Move me</button></section>",
        boilerplate()
    );
    store
        .update(
            document(session, &target_html),
            TransitionCause::ExternalChange,
        )
        .unwrap();
    let incremental = builder()
        .build(
            &store,
            &ObservationRequest::new(session).since_state(base).unwrap(),
        )
        .unwrap();
    let full_target = builder()
        .build(&store, &ObservationRequest::new(session))
        .unwrap();

    assert_eq!(
        incremental.observation().basis(),
        ObservationBasis::Incremental { base }
    );
    let ObservationChanges::Computed(changes) = incremental.observation().changes() else {
        panic!("same-page retained base must produce a computed diff");
    };
    assert!(changes.summary_changed());
    assert!(changes.order_changed());
    assert!(!changes.added().is_empty());
    assert!(!changes.updated().is_empty());
    assert!(!changes.removed().is_empty());
    assert!(changes.removed().contains(&old_role));
    assert!(changes.added().contains(&reference(&store, "role")));
    for id in [
        "parent-a", "expander", "text", "check", "one", "two", "link", "moved",
    ] {
        assert!(
            changes.updated().contains(&reference(&store, id)),
            "{id} must be emitted as an updated semantic unit"
        );
    }
    let stable = reference(&store, "stable-0");
    assert!(
        !changes
            .changed_references()
            .any(|reference| reference == stable)
    );
    assert!(incremental.observation().units().len() < full_target.observation().units().len());
    assert!(
        incremental.diagnostics().emitted_logical_content_bytes()
            < incremental.diagnostics().logical_content_bytes()
    );

    let reconstructed = SemanticSnapshot::from_full(&base_observation)
        .unwrap()
        .apply(incremental.observation())
        .unwrap();
    let expected = SemanticSnapshot::from_full(full_target.observation()).unwrap();
    assert_eq!(reconstructed, expected);
}

#[test]
fn identical_and_generated_transitions_reconstruct_deterministically() {
    for seed in 0_u64..16 {
        let session = SessionId::new(100 + seed).unwrap();
        let mut store = store(session, StateStoreConfig::default());
        let base_items = (0..8)
            .map(|index| format!("<p id='item-{index}'>Value {index}</p>"))
            .collect::<String>();
        let base_html = format!("<!doctype html><title>Generated</title>{base_items}");
        let base = store
            .update(document(session, &base_html), TransitionCause::Refresh)
            .unwrap()
            .transition()
            .to();
        let base_observation = builder()
            .build(&store, &ObservationRequest::new(session))
            .unwrap()
            .into_observation();

        let mut indexes = (0..8).collect::<Vec<_>>();
        indexes.rotate_left((seed as usize) % 8);
        let target_items = indexes
            .into_iter()
            .filter(|index| !(*index as u64 + seed).is_multiple_of(5))
            .map(|index| {
                let suffix = if (index as u64 + seed).is_multiple_of(3) {
                    " changed"
                } else {
                    ""
                };
                format!("<p id='item-{index}'>Value {index}{suffix}</p>")
            })
            .collect::<String>();
        let target_html = format!(
            "<!doctype html><title>Generated</title>{target_items}<p id='new-{seed}'>New</p>"
        );
        store
            .update(
                document(session, &target_html),
                TransitionCause::ExternalChange,
            )
            .unwrap();
        let request = ObservationRequest::new(session).since_state(base).unwrap();
        let first = builder().build(&store, &request).unwrap();
        let second = builder().build(&store, &request).unwrap();
        assert_eq!(
            first.observation().changes(),
            second.observation().changes()
        );
        assert_eq!(first.observation().units(), second.observation().units());
        assert_eq!(
            first.observation().semantic_order(),
            second.observation().semantic_order()
        );

        let reconstructed = SemanticSnapshot::from_full(&base_observation)
            .unwrap()
            .apply(first.observation())
            .unwrap();
        let full_target = builder()
            .build(&store, &ObservationRequest::new(session))
            .unwrap();
        assert_eq!(
            reconstructed,
            SemanticSnapshot::from_full(full_target.observation()).unwrap()
        );
    }

    let session = SessionId::new(200).unwrap();
    let mut store = store(session, StateStoreConfig::default());
    let html = "<!doctype html><title>Same</title><button id='same'>Same</button>";
    let base = store
        .update(document(session, html), TransitionCause::Refresh)
        .unwrap()
        .transition()
        .to();
    let base_observation = builder()
        .build(&store, &ObservationRequest::new(session))
        .unwrap()
        .into_observation();
    store
        .update(document(session, html), TransitionCause::ExternalChange)
        .unwrap();
    let incremental = builder()
        .build(
            &store,
            &ObservationRequest::new(session).since_state(base).unwrap(),
        )
        .unwrap();
    let ObservationChanges::Computed(changes) = incremental.observation().changes() else {
        panic!("identical retained states must produce an empty computed diff");
    };
    assert_eq!(changes.unit_change_count(), 0);
    assert!(!changes.summary_changed());
    assert!(!changes.order_changed());
    assert!(incremental.observation().units().is_empty());
    assert!(incremental.observation().semantic_order().is_empty());
    let reconstructed = SemanticSnapshot::from_full(&base_observation)
        .unwrap()
        .apply(incremental.observation())
        .unwrap();
    let expected = builder()
        .build(&store, &ObservationRequest::new(session))
        .unwrap();
    assert_eq!(
        reconstructed,
        SemanticSnapshot::from_full(expected.observation()).unwrap()
    );
}

#[test]
fn oversized_diff_resets_to_a_complete_target_view() {
    let session = SessionId::new(32).unwrap();
    let mut store = store(session, StateStoreConfig::default());
    let base_items = (0..12)
        .map(|index| format!("<button id='item-{index}'>Item {index}</button>"))
        .collect::<String>();
    let base = store
        .update(
            document(
                session,
                &format!("<!doctype html><title>Bounded</title>{base_items}"),
            ),
            TransitionCause::Refresh,
        )
        .unwrap()
        .transition()
        .to();
    let target_items = (0..12)
        .rev()
        .map(|index| format!("<button id='item-{index}'>Changed {index}</button>"))
        .collect::<String>();
    store
        .update(
            document(
                session,
                &format!("<!doctype html><title>Bounded</title>{target_items}"),
            ),
            TransitionCause::ExternalChange,
        )
        .unwrap();
    let limited = FullObservationBuilder::new(
        capabilities(&engine()),
        FullObservationConfig::default()
            .with_change_limit(CollectionLimit::new(5, "change_limit").unwrap()),
    );
    let reset = limited
        .build(
            &store,
            &ObservationRequest::new(session).since_state(base).unwrap(),
        )
        .unwrap();
    assert_eq!(
        reset.observation().basis(),
        ObservationBasis::Reset {
            requested_base: base,
            reason: ResetReason::DiffTooLarge,
        }
    );
    assert_eq!(
        reset.observation().changes(),
        &ObservationChanges::Reset {
            requested_base: base,
            reason: ResetReason::DiffTooLarge,
        }
    );
    assert_eq!(
        reset.observation().units().len(),
        store.current().unwrap().units().len()
    );
    assert_eq!(
        reset.observation().semantic_order().len(),
        reset.observation().units().len()
    );
    assert!(reset.diagnostics().diff().is_none());
    SemanticSnapshot::from_full(reset.observation()).unwrap();
}

#[test]
fn snapshot_rejects_wrong_base_and_non_incremental_input() {
    let session = SessionId::new(33).unwrap();
    let mut store = store(session, StateStoreConfig::default());
    let base = store
        .update(
            document(session, "<!doctype html><p id='value'>Before</p>"),
            TransitionCause::Refresh,
        )
        .unwrap()
        .transition()
        .to();
    let base_observation = builder()
        .build(&store, &ObservationRequest::new(session))
        .unwrap()
        .into_observation();
    store
        .update(
            document(session, "<!doctype html><p id='value'>After</p>"),
            TransitionCause::ExternalChange,
        )
        .unwrap();
    let incremental = builder()
        .build(
            &store,
            &ObservationRequest::new(session).since_state(base).unwrap(),
        )
        .unwrap();
    let wrong_session = SessionId::new(34).unwrap();
    let wrong_base = mawr_core::Observation::new(
        mawr_core::StateId::new(wrong_session, 1).unwrap(),
        mawr_core::PageIdentity::new(
            mawr_core::PageId::new(wrong_session, 1).unwrap(),
            AbsoluteUrl::new("https://example.test/state").unwrap(),
        ),
        engine(),
        capabilities(&engine()),
        ObservationBasis::Full(mawr_core::FullObservationReason::Initial),
        CollectionLimit::new(10, "unit_limit").unwrap(),
    )
    .unwrap();
    let wrong_snapshot = SemanticSnapshot::from_full(&wrong_base).unwrap();
    assert!(wrong_snapshot.apply(incremental.observation()).is_err());

    let valid_snapshot = SemanticSnapshot::from_full(&base_observation).unwrap();
    assert!(valid_snapshot.apply(&base_observation).is_err());
    let changed = incremental
        .observation()
        .units()
        .iter()
        .map(|unit| unit.reference())
        .collect::<BTreeSet<_>>();
    assert!(!changed.is_empty());
}
