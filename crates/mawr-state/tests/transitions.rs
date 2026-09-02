use std::collections::{BTreeMap, BTreeSet};

use mawr_core::{
    AbsoluteUrl, EngineIdentity, EngineKind, FailureClass, OperationFailure, Property,
    RelationshipKind, ResetReason, SemanticRole, SessionId, TransitionCause,
};
use mawr_semantic_html::{
    HtmlDocumentSource, HtmlSemanticExtractor, SemanticDocument, SourceNodeId,
};
use mawr_state::{
    ReferenceAssignmentReason, ReferenceLossReason, SemanticStateStore, StateStoreConfig,
    StoredSemanticUnit, StoredState,
};

fn engine() -> EngineIdentity {
    EngineIdentity::new("native-static-html", "0", EngineKind::NativeStatic).unwrap()
}

fn document(session: SessionId, url: &str, html: &str) -> SemanticDocument {
    let url = AbsoluteUrl::new(url).unwrap();
    HtmlSemanticExtractor::default()
        .extract_source(HtmlDocumentSource::new(session, &url, html.as_bytes()))
        .unwrap()
}

fn store(session: SessionId) -> SemanticStateStore {
    SemanticStateStore::new(session, engine(), StateStoreConfig::default())
}

fn author_reference(state: &StoredState, author_id: &str) -> mawr_core::ElementRef {
    state
        .units()
        .iter()
        .find(|unit| {
            matches!(
                unit.semantic().author_id(),
                Property::Known(value) if value.as_str() == author_id
            )
        })
        .map(StoredSemanticUnit::reference)
        .unwrap()
}

fn named_reference(state: &StoredState, role: SemanticRole, name: &str) -> mawr_core::ElementRef {
    state
        .units()
        .iter()
        .find(|unit| {
            unit.semantic().role() == role
                && matches!(
                    unit.semantic().name(),
                    Property::Known(value) if value.as_str() == name
                )
        })
        .map(StoredSemanticUnit::reference)
        .unwrap()
}

#[test]
fn references_survive_insert_remove_reorder_text_and_value_changes() {
    let session = SessionId::new(7).unwrap();
    let mut store = store(session);
    let first = store
        .update(
            document(
                session,
                "https://example.test/account",
                include_str!("fixtures/transition-before.html"),
            ),
            TransitionCause::Refresh,
        )
        .unwrap();
    assert_eq!(first.transition().cause(), TransitionCause::Initial);
    let first_state = store.current().unwrap();
    let save = author_reference(first_state, "save");
    let email = author_reference(first_state, "email");
    let docs = named_reference(first_state, SemanticRole::Link, "Docs");
    let cancel = named_reference(first_state, SemanticRole::Button, "Cancel");
    let removed = author_reference(first_state, "remove");

    let second = store
        .update(
            document(
                session,
                "https://example.test/account",
                include_str!("fixtures/transition-after.html"),
            ),
            TransitionCause::ExternalChange,
        )
        .unwrap();
    let second_state = store.current().unwrap();
    assert_eq!(author_reference(second_state, "save"), save);
    assert_eq!(author_reference(second_state, "email"), email);
    assert_eq!(
        named_reference(second_state, SemanticRole::Link, "Docs"),
        docs
    );
    assert_eq!(
        named_reference(second_state, SemanticRole::Button, "Cancel"),
        cancel
    );
    assert_eq!(second.diagnostics().preserved_references(), 5);
    assert!(second.diagnostics().losses().iter().any(|loss| {
        loss.reference() == removed
            && loss.reason() == ReferenceLossReason::RemovedOrIdentityChanged
    }));
    assert_eq!(second.transition().from(), Some(first.transition().to()));
}

#[test]
fn duplicate_semantic_elements_and_author_ids_fail_closed() {
    let session = SessionId::new(8).unwrap();
    let mut semantic_store = store(session);
    let duplicate_semantic = include_str!("fixtures/duplicate-semantic.html");
    semantic_store
        .update(
            document(session, "https://example.test/semantic", duplicate_semantic),
            TransitionCause::Refresh,
        )
        .unwrap();
    let original = semantic_store
        .current()
        .unwrap()
        .units()
        .iter()
        .filter(|unit| unit.semantic().role() == SemanticRole::Button)
        .map(StoredSemanticUnit::reference)
        .collect::<BTreeSet<_>>();
    let update = semantic_store
        .update(
            document(session, "https://example.test/semantic", duplicate_semantic),
            TransitionCause::Refresh,
        )
        .unwrap();
    let current = semantic_store
        .current()
        .unwrap()
        .units()
        .iter()
        .filter(|unit| unit.semantic().role() == SemanticRole::Button)
        .map(StoredSemanticUnit::reference)
        .collect::<BTreeSet<_>>();
    assert!(original.is_disjoint(&current));
    assert_eq!(
        update
            .diagnostics()
            .assignments()
            .iter()
            .filter(|assignment| {
                assignment.reason() == ReferenceAssignmentReason::AmbiguousSemanticIdentity
            })
            .count(),
        2
    );

    let mut id_store = store(session);
    let duplicate_ids = include_str!("fixtures/duplicate-author-id.html");
    id_store
        .update(
            document(session, "https://example.test/ids", duplicate_ids),
            TransitionCause::Refresh,
        )
        .unwrap();
    let update = id_store
        .update(
            document(session, "https://example.test/ids", duplicate_ids),
            TransitionCause::Refresh,
        )
        .unwrap();
    assert_eq!(
        update
            .diagnostics()
            .assignments()
            .iter()
            .filter(|assignment| {
                assignment.reason() == ReferenceAssignmentReason::AmbiguousAuthorId
            })
            .count(),
        2
    );
}

#[test]
fn navigation_resets_page_and_element_identity() {
    let session = SessionId::new(9).unwrap();
    let html = "<!doctype html><title>Page</title><button id='go'>Go</button>";
    let mut store = store(session);
    store
        .update(
            document(session, "https://example.test/one", html),
            TransitionCause::Refresh,
        )
        .unwrap();
    let first_page = store.current().unwrap().page().id();
    let first_reference = author_reference(store.current().unwrap(), "go");
    let update = store
        .update(
            document(session, "https://example.test/two", html),
            TransitionCause::Refresh,
        )
        .unwrap();
    assert_eq!(update.transition().cause(), TransitionCause::Navigation);
    assert_eq!(
        update.diagnostics().reset(),
        Some(ResetReason::NavigationBoundary)
    );
    assert_ne!(store.current().unwrap().page().id(), first_page);
    assert_ne!(
        author_reference(store.current().unwrap(), "go"),
        first_reference
    );
    assert!(update.diagnostics().losses().iter().all(|loss| {
        loss.reason() == ReferenceLossReason::Reset(ResetReason::NavigationBoundary)
    }));
}

#[test]
fn retention_eviction_and_current_lookup_are_explicit() {
    let session = SessionId::new(10).unwrap();
    let config = StateStoreConfig::default()
        .with_retained_states(2)
        .unwrap()
        .with_retained_units(20)
        .unwrap();
    let mut store = SemanticStateStore::new(session, engine(), config);
    let html = "<!doctype html><title>Page</title><button id='go'>Go</button>";
    let first = store
        .update(
            document(session, "https://example.test", html),
            TransitionCause::Refresh,
        )
        .unwrap();
    let first_id = first.transition().to();
    let reference = author_reference(store.current().unwrap(), "go");
    let second = store
        .update(
            document(session, "https://example.test", html),
            TransitionCause::Refresh,
        )
        .unwrap();
    let second_id = second.transition().to();
    let third = store
        .update(
            document(session, "https://example.test", html),
            TransitionCause::Refresh,
        )
        .unwrap();

    assert_eq!(third.diagnostics().evicted_states(), &[first_id]);
    assert_eq!(store.retained_state_ids().collect::<Vec<_>>().len(), 2);
    assert!(matches!(
        store.state(first_id),
        Err(OperationFailure::StaleState { .. })
    ));
    assert!(store.state(second_id).is_ok());
    assert!(matches!(
        store.resolve_current(second_id, reference),
        Err(OperationFailure::StaleState { .. })
    ));
    assert!(
        store
            .resolve_current(third.transition().to(), reference)
            .is_ok()
    );
    let missing = mawr_core::ElementRef::new(session, u32::MAX).unwrap();
    assert!(matches!(
        store.resolve_current(third.transition().to(), missing),
        Err(OperationFailure::MissingReference { .. })
    ));
}

#[test]
fn sessions_and_memory_limits_are_enforced() {
    let first_session = SessionId::new(11).unwrap();
    let second_session = SessionId::new(12).unwrap();
    let html = "<!doctype html><title>Page</title><button id='go'>Go</button>";
    let mut first = store(first_session);
    let mut second = store(second_session);
    let first_update = first
        .update(
            document(first_session, "https://example.test", html),
            TransitionCause::Refresh,
        )
        .unwrap();
    second
        .update(
            document(second_session, "https://example.test", html),
            TransitionCause::Refresh,
        )
        .unwrap();
    let first_reference = author_reference(first.current().unwrap(), "go");
    let second_reference = author_reference(second.current().unwrap(), "go");
    assert_ne!(first_reference, second_reference);
    let failure = first
        .resolve_current(first_update.transition().to(), second_reference)
        .unwrap_err();
    assert_eq!(failure.class(), FailureClass::InvalidInput);

    let config = StateStoreConfig::default().with_retained_units(2).unwrap();
    let mut limited = SemanticStateStore::new(first_session, engine(), config);
    let too_large = "<!doctype html><title>Page</title><button>One</button><button>Two</button>";
    let failure = limited
        .update(
            document(first_session, "https://example.test", too_large),
            TransitionCause::Refresh,
        )
        .unwrap_err();
    assert_eq!(failure.class(), FailureClass::ResourceLimit);
    assert!(limited.current().is_none());
}

#[test]
fn assignment_is_deterministic_and_relationships_use_stable_targets() {
    let session = SessionId::new(13).unwrap();
    let fixture = include_str!("fixtures/relationships.html");
    let mut first = store(session);
    let mut second = store(session);
    let first_update = first
        .update(
            document(session, "https://example.test/tasks", fixture),
            TransitionCause::Refresh,
        )
        .unwrap();
    let second_update = second
        .update(
            document(session, "https://example.test/tasks", fixture),
            TransitionCause::Refresh,
        )
        .unwrap();
    let first_assignments = first_update
        .diagnostics()
        .assignments()
        .iter()
        .map(|assignment| {
            (
                assignment.source(),
                assignment.reference(),
                assignment.reason(),
            )
        })
        .collect::<Vec<(
            SourceNodeId,
            mawr_core::ElementRef,
            ReferenceAssignmentReason,
        )>>();
    let second_assignments = second_update
        .diagnostics()
        .assignments()
        .iter()
        .map(|assignment| {
            (
                assignment.source(),
                assignment.reference(),
                assignment.reason(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(first_assignments, second_assignments);

    let state = first.current().unwrap();
    let refs_by_role = state.units().iter().fold(
        BTreeMap::<SemanticRole, Vec<mawr_core::ElementRef>>::new(),
        |mut map, unit| {
            map.entry(unit.semantic().role())
                .or_default()
                .push(unit.reference());
            map
        },
    );
    let list_reference = refs_by_role[&SemanticRole::List][0];
    for item in state
        .units()
        .iter()
        .filter(|unit| unit.semantic().role() == SemanticRole::ListItem)
    {
        assert!(item.relationships().iter().any(|relationship| {
            relationship.kind() == RelationshipKind::ListItemOf
                && relationship.target() == list_reference
        }));
    }
}

#[test]
fn explicit_reset_invalidates_same_url_references() {
    let session = SessionId::new(14).unwrap();
    let html = "<!doctype html><title>Page</title><button id='go'>Go</button>";
    let mut store = store(session);
    store
        .update(
            document(session, "https://example.test", html),
            TransitionCause::Refresh,
        )
        .unwrap();
    let reference = author_reference(store.current().unwrap(), "go");
    let update = store
        .update(
            document(session, "https://example.test", html),
            TransitionCause::Reset(ResetReason::ExplicitRequest),
        )
        .unwrap();
    assert_eq!(
        update.diagnostics().reset(),
        Some(ResetReason::ExplicitRequest)
    );
    assert_ne!(author_reference(store.current().unwrap(), "go"), reference);
}

#[test]
fn rejected_transition_does_not_mutate_store_sequences() {
    let session = SessionId::new(15).unwrap();
    let html = "<!doctype html><title>Page</title><button id='go'>Go</button>";
    let mut store = store(session);
    let first = store
        .update(
            document(session, "https://example.test", html),
            TransitionCause::Refresh,
        )
        .unwrap();
    let failure = store
        .update(
            document(session, "https://example.test", html),
            TransitionCause::Initial,
        )
        .unwrap_err();
    assert_eq!(failure.class(), FailureClass::InvalidInput);
    assert_eq!(store.current().unwrap().id(), first.transition().to());

    let second = store
        .update(
            document(session, "https://example.test", html),
            TransitionCause::Refresh,
        )
        .unwrap();
    assert_eq!(second.transition().to().sequence(), 2);
}

#[test]
fn synthetic_transition_reports_survival_and_bounded_retention_diagnostics() {
    let session = SessionId::new(16).unwrap();
    let mut before = String::from("<!doctype html><title>Synthetic</title>");
    let mut after = String::from("<!doctype html><title>Synthetic changed</title>");
    for index in 0..512 {
        before.push_str(&format!(
            "<input id='field-{index}' aria-label='Field {index}' value='before'>"
        ));
    }
    for index in (0..512).rev() {
        after.push_str(&format!(
            "<input id='field-{index}' aria-label='Changed {index}' value='after'>"
        ));
    }
    let config = StateStoreConfig::default()
        .with_retained_states(2)
        .unwrap()
        .with_retained_units(1_100)
        .unwrap();
    let mut store = SemanticStateStore::new(session, engine(), config);
    store
        .update(
            document(session, "https://example.test/synthetic", &before),
            TransitionCause::Refresh,
        )
        .unwrap();
    let update = store
        .update(
            document(session, "https://example.test/synthetic", &after),
            TransitionCause::ExternalChange,
        )
        .unwrap();

    assert_eq!(update.diagnostics().preserved_references(), 513);
    assert_eq!(update.diagnostics().new_references(), 0);
    assert_eq!(update.diagnostics().retained_states(), 2);
    assert_eq!(update.diagnostics().retained_units(), 1_026);
    let _measured_matching_latency = update.diagnostics().matching_latency_micros();
}
