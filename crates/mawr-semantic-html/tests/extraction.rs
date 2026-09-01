use mawr_core::{
    AbsoluteUrl, ActionKind, Measurement, MeasurementKind, Property, PropertyUnknownReason,
    RelationshipKind, ResourceKind, SemanticRole, SemanticValue, SessionId, UnavailableReason,
};
use mawr_semantic_html::{
    ExtractionLimits, ExtractionNoticeKind, HtmlDocumentSource, HtmlSemanticExtractor,
    SemanticDocument,
};

fn extract(html: &str) -> SemanticDocument {
    let session = SessionId::new(7).unwrap();
    let url = AbsoluteUrl::new("https://example.test/root/page").unwrap();
    HtmlSemanticExtractor::default()
        .extract_source(
            HtmlDocumentSource::new(session, &url, html.as_bytes())
                .with_content_type(Some("text/html; charset=utf-8")),
        )
        .unwrap()
}

fn named<'a>(
    document: &'a SemanticDocument,
    role: SemanticRole,
    name: &str,
) -> &'a mawr_semantic_html::ExtractedSemanticUnit {
    document
        .units()
        .iter()
        .find(|unit| {
            unit.role() == role
                && matches!(unit.name(), Property::Known(value) if value.as_str() == name)
        })
        .unwrap_or_else(|| panic!("missing {role:?} named {name:?}"))
}

#[test]
fn controls_have_names_states_values_relationships_urls_and_affordances() {
    let document = extract(include_str!("fixtures/controls.html"));
    assert_eq!(document.title(), Some("Accesso MAWR"));
    assert_eq!(document.language(), Some("it"));
    assert_eq!(document.base_url().as_str(), "https://example.test/app/");

    let email = named(&document, SemanticRole::Textbox, "E-mail");
    assert_eq!(email.state().required(), &Property::Known(true));
    assert!(email.affordances().contains(ActionKind::Fill));
    assert!(
        email
            .relationships()
            .iter()
            .any(|relationship| relationship.kind() == RelationshipKind::DescribedBy)
    );

    let password = named(&document, SemanticRole::Textbox, "Password");
    assert_eq!(password.value(), &SemanticValue::Redacted);
    assert!(!format!("{document:?}").contains("super-secret"));

    let remember = named(&document, SemanticRole::Checkbox, "Ricordami");
    assert_eq!(remember.state().checked(), &Property::Known(true));
    assert!(remember.affordances().contains(ActionKind::Uncheck));

    let link = named(&document, SemanticRole::Link, "Privacy");
    assert!(link.affordances().contains(ActionKind::Follow));
    assert!(
        matches!(link.destination(), Property::Known(url) if url.as_str() == "https://example.test/privacy")
    );

    let disabled = named(&document, SemanticRole::Button, "Non disponibile");
    assert_eq!(disabled.state().disabled(), &Property::Known(true));
    assert_eq!(disabled.affordances().iter().count(), 0);
    assert!(!document.units().iter().any(
        |unit| matches!(unit.name(), Property::Known(value) if value.as_str() == "Invisibile")
    ));

    let external_submit = named(&document, SemanticRole::Button, "Invia fuori");
    assert!(external_submit.affordances().contains(ActionKind::Submit));
    assert!(
        matches!(external_submit.destination(), Property::Known(url) if url.as_str() == "https://example.test/app/session")
    );
    assert!(
        external_submit
            .relationships()
            .iter()
            .any(|relationship| relationship.kind() == RelationshipKind::OwnedBy)
    );
}

#[test]
fn tables_lists_and_options_keep_structural_relationships() {
    let document = extract(include_str!("fixtures/structures.html"));
    assert_eq!(
        document
            .units()
            .iter()
            .filter(|unit| unit.role() == SemanticRole::Row)
            .count(),
        2
    );
    assert_eq!(
        document
            .units()
            .iter()
            .filter(|unit| unit.role() == SemanticRole::Cell)
            .count(),
        4
    );
    assert_eq!(
        document
            .units()
            .iter()
            .filter(|unit| unit.role() == SemanticRole::ListItem)
            .count(),
        2
    );
    assert!(
        document
            .units()
            .iter()
            .filter(|unit| unit.role() == SemanticRole::Cell)
            .all(|unit| unit
                .relationships()
                .iter()
                .any(|relationship| relationship.kind() == RelationshipKind::CellOf))
    );
    let option = named(&document, SemanticRole::Option, "M");
    assert_eq!(option.state().selected(), &Property::Known(true));
    assert!(
        option
            .relationships()
            .iter()
            .any(|relationship| relationship.kind() == RelationshipKind::OptionOf)
    );
}

#[test]
fn malformed_and_nested_forms_are_repaired_deterministically() {
    let first = extract(include_str!("fixtures/malformed.html"));
    let second = extract(include_str!("fixtures/malformed.html"));
    assert_eq!(first.units(), second.units());
    assert_eq!(first.notices(), second.notices());
    assert!(
        first
            .units()
            .iter()
            .any(|unit| unit.role() == SemanticRole::Form)
    );
    assert!(
        first
            .units()
            .iter()
            .any(|unit| unit.role() == SemanticRole::Table)
    );
    assert!(
        first
            .units()
            .iter()
            .any(|unit| unit.role() == SemanticRole::List)
    );
}

#[test]
fn ambiguity_and_unsupported_urls_are_explicit() {
    let document = extract(include_str!("fixtures/ambiguous.html"));
    let button = document
        .units()
        .iter()
        .find(|unit| unit.role() == SemanticRole::Button)
        .unwrap();
    assert_eq!(
        button.name(),
        &Property::Unknown(PropertyUnknownReason::Ambiguous)
    );
    let link = named(&document, SemanticRole::Link, "Non eseguire");
    assert_eq!(
        link.destination(),
        &Property::Unknown(PropertyUnknownReason::Unsupported)
    );
    assert!(!link.affordances().contains(ActionKind::Follow));
    assert!(
        document
            .notices()
            .iter()
            .any(|notice| notice.kind() == ExtractionNoticeKind::DuplicateHtmlId)
    );
    assert!(
        document
            .notices()
            .iter()
            .any(|notice| notice.kind() == ExtractionNoticeKind::BrokenIdReference)
    );
    assert!(
        document
            .notices()
            .iter()
            .any(|notice| notice.kind() == ExtractionNoticeKind::UnsupportedUrlScheme)
    );
    assert!(document.units().iter().any(
        |unit| matches!(unit.name(), Property::Known(value) if value.as_str().contains("日本語"))
    ));
    assert_eq!(document.units().iter().filter(|unit| matches!(unit.name(), Property::Known(value) if value.as_str() == "Duplicato")).count(), 2);
}

#[test]
fn parser_resource_limits_fail_closed() {
    let session = SessionId::new(1).unwrap();
    let url = AbsoluteUrl::new("https://example.test/").unwrap();
    let deep = format!("{}x{}", "<div>".repeat(80), "</div>".repeat(80));
    let extractor =
        HtmlSemanticExtractor::new(ExtractionLimits::default().with_dom_depth(16).unwrap());
    let error = extractor
        .extract_source(HtmlDocumentSource::new(session, &url, deep.as_bytes()))
        .unwrap_err();
    assert!(matches!(
        error,
        mawr_core::OperationFailure::ResourceLimit {
            resource: ResourceKind::DomDepth,
            ..
        }
    ));

    let extractor =
        HtmlSemanticExtractor::new(ExtractionLimits::default().with_document_bytes(32).unwrap());
    let error = extractor
        .extract_source(HtmlDocumentSource::new(
            session,
            &url,
            include_str!("fixtures/controls.html").as_bytes(),
        ))
        .unwrap_err();
    assert!(matches!(
        error,
        mawr_core::OperationFailure::ResourceLimit {
            resource: ResourceKind::ResponseBytes,
            ..
        }
    ));

    let wide = format!("<body>{}</body>", "<span>x</span>".repeat(100));
    let extractor =
        HtmlSemanticExtractor::new(ExtractionLimits::default().with_dom_nodes(20).unwrap());
    let error = extractor
        .extract_source(HtmlDocumentSource::new(session, &url, wide.as_bytes()))
        .unwrap_err();
    assert!(matches!(
        error,
        mawr_core::OperationFailure::ResourceLimit {
            resource: ResourceKind::DomNodes,
            ..
        }
    ));

    let units = format!("<body>{}</body>", "<p>x</p>".repeat(20));
    let extractor =
        HtmlSemanticExtractor::new(ExtractionLimits::default().with_semantic_units(5).unwrap());
    let error = extractor
        .extract_source(HtmlDocumentSource::new(session, &url, units.as_bytes()))
        .unwrap_err();
    assert!(matches!(
        error,
        mawr_core::OperationFailure::ResourceLimit {
            resource: ResourceKind::SemanticUnits,
            ..
        }
    ));

    let extractor = HtmlSemanticExtractor::new(
        ExtractionLimits::default()
            .with_relationships_per_unit(2)
            .unwrap(),
    );
    let relationships = r#"<span id="a">A</span><span id="b">B</span><span id="c">C</span><button aria-describedby="a b c">Go</button>"#;
    let error = extractor
        .extract_source(HtmlDocumentSource::new(
            session,
            &url,
            relationships.as_bytes(),
        ))
        .unwrap_err();
    assert!(matches!(
        error,
        mawr_core::OperationFailure::ResourceLimit {
            resource: ResourceKind::SemanticRelationships,
            ..
        }
    ));
}

#[test]
fn non_html_transport_metadata_is_rejected() {
    let session = SessionId::new(1).unwrap();
    let url = AbsoluteUrl::new("https://example.test/data").unwrap();
    let error = HtmlSemanticExtractor::default()
        .extract_source(
            HtmlDocumentSource::new(session, &url, br#"{"looks":"like html"}"#)
                .with_content_type(Some("application/json")),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        mawr_core::OperationFailure::ParsingFailure(mawr_core::ParsingFailureKind::InvalidDocument)
    ));
}

#[test]
fn native_default_selection_and_disabled_fieldset_legend_are_modeled() {
    let document = extract(
        r#"<fieldset disabled>
            <legend><button>Legend action</button></legend>
            <input aria-label="Blocked">
        </fieldset>
        <select aria-label="Choice"><option>First</option><option>Second</option></select>"#,
    );
    assert_eq!(
        named(&document, SemanticRole::Button, "Legend action")
            .state()
            .disabled(),
        &Property::Known(false)
    );
    assert_eq!(
        named(&document, SemanticRole::Textbox, "Blocked")
            .state()
            .disabled(),
        &Property::Known(true)
    );
    assert_eq!(
        named(&document, SemanticRole::Option, "First")
            .state()
            .selected(),
        &Property::Known(true)
    );
}

#[test]
fn fixed_diagnostic_corpus_reports_exact_counts_and_honest_measurements() {
    let fixtures = [
        include_str!("fixtures/diagnostic-small.html"),
        include_str!("fixtures/diagnostic-table.html"),
        include_str!("fixtures/diagnostic-boilerplate.html"),
        include_str!("fixtures/diagnostic-irrelevant.html"),
    ];
    for fixture in fixtures {
        let document = extract(fixture);
        assert!(document.diagnostics().dom_nodes() > 0);
        assert_eq!(
            document.diagnostics().semantic_units(),
            document.units().len() as u64
        );
        assert!(matches!(
            document
                .diagnostics()
                .measurements()
                .get(MeasurementKind::LatencyMicros),
            Measurement::Exact { .. }
        ));
        assert_eq!(
            document
                .diagnostics()
                .measurements()
                .get(MeasurementKind::CpuMicros),
            &Measurement::Unavailable(UnavailableReason::SourceMissing)
        );
        assert_eq!(
            document
                .diagnostics()
                .measurements()
                .get(MeasurementKind::PeakMemoryBytes),
            &Measurement::Unavailable(UnavailableReason::SourceMissing)
        );
    }
}
