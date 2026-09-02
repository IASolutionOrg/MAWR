use mawr_core::{
    AbsoluteUrl, ActionKind, BoundedText, CapabilityReport, CollectionLimit, ElementRef,
    ElementState, EngineIdentity, EngineKind, FullObservationReason, Measurement, MeasurementKind,
    Observation, ObservationBasis, ObservationRequest, ObservationTokenBudget, OmissionCategory,
    PageId, PageIdentity, Property, Provenance, SemanticRole, SemanticUnit, SemanticValue,
    SessionId, StateId, UnsupportedReason,
};
use mawr_relevance::{
    RankingConfig, RankingWeights, RelevanceSelector, SelectionContext, TokenCountQuality,
    TokenCounter, TokenizerMetadata, Utf8ByteEstimator,
};

struct FixedFragmentTokenizer {
    metadata: TokenizerMetadata,
    tokens: u64,
}

impl FixedFragmentTokenizer {
    fn new(tokens: u64) -> Self {
        Self {
            metadata: TokenizerMetadata::new("fixed-fragment", "1", TokenCountQuality::Exact)
                .unwrap(),
            tokens,
        }
    }
}

impl TokenCounter for FixedFragmentTokenizer {
    fn metadata(&self) -> &TokenizerMetadata {
        &self.metadata
    }

    fn count_tokens(&self, input: &str) -> u64 {
        if input.is_empty() { 0 } else { self.tokens }
    }
}

struct ScalarTokenizer {
    metadata: TokenizerMetadata,
}

impl ScalarTokenizer {
    fn new() -> Self {
        Self {
            metadata: TokenizerMetadata::new("unicode-scalars", "1", TokenCountQuality::Exact)
                .unwrap(),
        }
    }
}

impl TokenCounter for ScalarTokenizer {
    fn metadata(&self) -> &TokenizerMetadata {
        &self.metadata
    }

    fn count_tokens(&self, input: &str) -> u64 {
        input.chars().count() as u64
    }
}

fn engine() -> EngineIdentity {
    EngineIdentity::new("native-static", "0", EngineKind::NativeStatic).unwrap()
}

fn reference(session: SessionId, sequence: u32) -> ElementRef {
    ElementRef::new(session, sequence).unwrap()
}

fn unit(session: SessionId, sequence: u32, role: SemanticRole, name: &str) -> SemanticUnit {
    SemanticUnit::new(
        reference(session, sequence),
        role,
        Provenance::UntrustedWebContent,
    )
    .with_name(name)
    .unwrap()
}

fn page(session: SessionId) -> SemanticUnit {
    unit(session, 1, SemanticRole::Page, "Fixture page")
}

fn observation(session: SessionId, units: Vec<SemanticUnit>) -> Observation {
    let engine = engine();
    Observation::new(
        StateId::new(session, 1).unwrap(),
        PageIdentity::new(
            PageId::new(session, 1).unwrap(),
            AbsoluteUrl::new("https://example.test/fixture").unwrap(),
        ),
        engine.clone(),
        CapabilityReport::unsupported_all(engine, UnsupportedReason::NotImplemented),
        ObservationBasis::Full(FullObservationReason::Initial),
        CollectionLimit::new(10_000, "unit_limit").unwrap(),
    )
    .unwrap()
    .with_summary("Fixture")
    .unwrap()
    .with_units(units)
    .unwrap()
}

fn budgeted(session: SessionId, goal: &str, tokens: u64) -> ObservationRequest {
    ObservationRequest::new(session)
        .with_goal(goal)
        .unwrap()
        .with_max_tokens(ObservationTokenBudget::new(tokens, "max_tokens").unwrap())
}

fn selected_references(observation: &Observation) -> Vec<ElementRef> {
    observation
        .units()
        .iter()
        .map(SemanticUnit::reference)
        .collect()
}

fn zero_weights() -> RankingWeights {
    RankingWeights {
        goal_name_per_term: 0,
        goal_description_per_term: 0,
        goal_value_per_term: 0,
        interactive: 0,
        structural: 0,
        alert: 0,
        invalid: 0,
        changed: 0,
        context: 0,
        boilerplate_penalty: 0,
        repeated_navigation_penalty: 0,
    }
}

fn score_for(result: &mawr_relevance::RankedObservation, reference: ElementRef) -> i64 {
    result
        .diagnostics()
        .trace()
        .iter()
        .find(|trace| trace.reference() == reference)
        .unwrap()
        .score()
}

#[test]
fn ranking_signals_are_isolated_by_configurable_weights() {
    let session = SessionId::new(30).unwrap();
    let name = unit(session, 2, SemanticRole::Text, "Needle");
    let description = unit(session, 3, SemanticRole::Text, "Other").with_description_property(
        Property::Known(BoundedText::new("Needle", "description").unwrap()),
    );
    let value = unit(session, 4, SemanticRole::Text, "Other")
        .with_value(SemanticValue::text("Needle").unwrap());
    let heading = unit(session, 5, SemanticRole::Heading, "Other");
    let button = unit(session, 6, SemanticRole::Button, "Other").with_affordance(ActionKind::Press);
    let changed = unit(session, 7, SemanticRole::Text, "Other");
    let alert = unit(session, 8, SemanticRole::Alert, "Other");
    let invalid = unit(session, 9, SemanticRole::Textbox, "Other")
        .with_state(ElementState::new().with_invalid(Property::Known(true)));
    let subject = observation(
        session,
        vec![
            page(session),
            name,
            description,
            value,
            heading,
            button,
            changed,
            alert,
            invalid,
        ],
    );
    let mut weights = zero_weights();
    weights.goal_name_per_term = 11;
    weights.goal_description_per_term = 13;
    weights.goal_value_per_term = 17;
    weights.structural = 19;
    weights.interactive = 23;
    weights.changed = 29;
    weights.alert = 31;
    weights.invalid = 37;
    let result = RelevanceSelector::new(
        RankingConfig::new("isolated-signals")
            .unwrap()
            .with_weights(weights),
    )
    .select(
        &subject,
        &ObservationRequest::new(session)
            .with_goal("needle")
            .unwrap(),
        &FixedFragmentTokenizer::new(1),
        &SelectionContext::default().with_changed(reference(session, 7)),
    )
    .unwrap();

    assert_eq!(score_for(&result, reference(session, 2)), 11);
    assert_eq!(score_for(&result, reference(session, 3)), 13);
    assert_eq!(score_for(&result, reference(session, 4)), 17);
    assert_eq!(score_for(&result, reference(session, 5)), 19);
    assert_eq!(score_for(&result, reference(session, 6)), 23);
    assert_eq!(score_for(&result, reference(session, 7)), 29);
    assert_eq!(score_for(&result, reference(session, 8)), 31);
    assert_eq!(score_for(&result, reference(session, 9)), 37);
}

#[test]
fn goal_and_interactive_signals_rank_deterministically_and_ties_use_reference_order() {
    let session = SessionId::new(31).unwrap();
    let first = unit(session, 2, SemanticRole::Button, "Submit order")
        .with_parent(reference(session, 1))
        .unwrap()
        .with_affordance(ActionKind::Press);
    let second = unit(session, 3, SemanticRole::Button, "Submit order")
        .with_parent(reference(session, 1))
        .unwrap()
        .with_affordance(ActionKind::Press);
    let subject = observation(session, vec![page(session), first, second]);
    let request = budgeted(session, "submit", 3);
    let tokenizer = FixedFragmentTokenizer::new(1);

    let first_run = RelevanceSelector::default()
        .select(&subject, &request, &tokenizer, &SelectionContext::default())
        .unwrap();
    let second_run = RelevanceSelector::default()
        .select(&subject, &request, &tokenizer, &SelectionContext::default())
        .unwrap();

    assert_eq!(
        selected_references(first_run.observation()),
        vec![reference(session, 1), reference(session, 2)]
    );
    assert_eq!(
        selected_references(first_run.observation()),
        selected_references(second_run.observation())
    );
    assert_eq!(
        first_run.diagnostics().trace(),
        second_run.diagnostics().trace()
    );
    let trace = first_run
        .diagnostics()
        .trace()
        .iter()
        .find(|trace| trace.reference() == reference(session, 2))
        .unwrap();
    assert_eq!(trace.signals().name_overlap(), 1);
    assert!(trace.signals().interactive());
    assert_eq!(
        first_run
            .observation()
            .omissions()
            .count(OmissionCategory::Budget),
        1
    );
}

#[test]
fn tokenizer_quality_and_unicode_variance_are_explicit() {
    let session = SessionId::new(32).unwrap();
    let subject = observation(
        session,
        vec![
            page(session),
            unit(session, 2, SemanticRole::Text, "Résumé 東京"),
        ],
    );
    let request = ObservationRequest::new(session);
    let estimated = Utf8ByteEstimator::new();
    let exact = ScalarTokenizer::new();
    assert_ne!(estimated.count_tokens("ééé"), exact.count_tokens("ééé"));

    let estimated_result = RelevanceSelector::default()
        .select(&subject, &request, &estimated, &SelectionContext::default())
        .unwrap();
    let exact_result = RelevanceSelector::default()
        .select(&subject, &request, &exact, &SelectionContext::default())
        .unwrap();
    assert_eq!(
        estimated_result.diagnostics().tokenizer().quality(),
        TokenCountQuality::Estimated
    );
    assert_eq!(
        exact_result.diagnostics().tokenizer().quality(),
        TokenCountQuality::Exact
    );
    assert!(matches!(
        estimated_result
            .observation()
            .measurements()
            .get(MeasurementKind::ObservationTokens),
        Measurement::Estimated { .. }
    ));
    assert!(matches!(
        exact_result
            .observation()
            .measurements()
            .get(MeasurementKind::ObservationTokens),
        Measurement::Exact { .. }
    ));
}

#[test]
fn zero_available_budget_keeps_oversized_essential_units_and_reports_overshoot() {
    let session = SessionId::new(33).unwrap();
    let alert = unit(session, 2, SemanticRole::Alert, "Payment failed")
        .with_parent(reference(session, 1))
        .unwrap();
    let ordinary = unit(session, 3, SemanticRole::Text, "Background copy")
        .with_parent(reference(session, 1))
        .unwrap();
    let subject = observation(session, vec![page(session), alert, ordinary]);
    let request = budgeted(session, "unrelated", 1);
    let selector = RelevanceSelector::new(RankingConfig::default().with_reserved_tokens(1));
    let result = selector
        .select(
            &subject,
            &request,
            &FixedFragmentTokenizer::new(50),
            &SelectionContext::default(),
        )
        .unwrap();

    assert_eq!(
        selected_references(result.observation()),
        vec![reference(session, 1), reference(session, 2)]
    );
    assert!(result.diagnostics().budget_overshoot_tokens() > 0);
    assert_eq!(result.diagnostics().reserved_tokens(), 1);
    assert_eq!(result.diagnostics().requested_budget(), Some(1));
}

#[test]
fn relevant_form_control_is_packed_with_its_structural_context() {
    let session = SessionId::new(34).unwrap();
    let form = unit(session, 2, SemanticRole::Form, "Checkout")
        .with_parent(reference(session, 1))
        .unwrap();
    let textbox = unit(session, 3, SemanticRole::Textbox, "Email address")
        .with_parent(reference(session, 2))
        .unwrap()
        .with_affordance(ActionKind::Fill);
    let noise = unit(session, 4, SemanticRole::Text, "Footer")
        .with_parent(reference(session, 1))
        .unwrap();
    let subject = observation(session, vec![page(session), form, textbox, noise]);
    let result = RelevanceSelector::default()
        .select(
            &subject,
            &budgeted(session, "email", 4),
            &FixedFragmentTokenizer::new(1),
            &SelectionContext::default(),
        )
        .unwrap();

    assert_eq!(
        selected_references(result.observation()),
        vec![
            reference(session, 1),
            reference(session, 2),
            reference(session, 3)
        ]
    );
    let form_trace = result
        .diagnostics()
        .trace()
        .iter()
        .find(|trace| trace.reference() == reference(session, 2))
        .unwrap();
    assert!(form_trace.signals().context());
}

#[test]
fn alerts_validation_and_changed_units_outrank_unchanged_controls() {
    let session = SessionId::new(35).unwrap();
    let alert = unit(session, 2, SemanticRole::Alert, "Error")
        .with_parent(reference(session, 1))
        .unwrap();
    let invalid = unit(session, 3, SemanticRole::Textbox, "Card number")
        .with_parent(reference(session, 1))
        .unwrap()
        .with_state(ElementState::new().with_invalid(Property::Known(true)))
        .with_affordance(ActionKind::Fill);
    let changed = unit(session, 4, SemanticRole::Button, "Continue")
        .with_parent(reference(session, 1))
        .unwrap()
        .with_affordance(ActionKind::Press);
    let unchanged = unit(session, 5, SemanticRole::Button, "Cancel")
        .with_parent(reference(session, 1))
        .unwrap()
        .with_affordance(ActionKind::Press);
    let subject = observation(
        session,
        vec![page(session), alert, invalid, changed, unchanged],
    );
    let context = SelectionContext::default().with_changed(reference(session, 4));
    let result = RelevanceSelector::default()
        .select(
            &subject,
            &budgeted(session, "unrelated", 5),
            &FixedFragmentTokenizer::new(1),
            &context,
        )
        .unwrap();

    let selected = selected_references(result.observation());
    assert!(selected.contains(&reference(session, 2)));
    assert!(selected.contains(&reference(session, 3)));
    assert!(selected.contains(&reference(session, 4)));
    assert!(!selected.contains(&reference(session, 5)));
    let changed_trace = result
        .diagnostics()
        .trace()
        .iter()
        .find(|trace| trace.reference() == reference(session, 4))
        .unwrap();
    assert!(changed_trace.signals().changed());
}

#[test]
fn repeated_navigation_and_thousands_of_irrelevant_units_are_omitted() {
    let session = SessionId::new(36).unwrap();
    let mut units = vec![page(session)];
    for sequence in 2..=1_001 {
        units.push(
            unit(session, sequence, SemanticRole::Link, "Home")
                .with_affordance(ActionKind::Follow)
                .with_destination(Property::Known(
                    AbsoluteUrl::new("https://example.test/home").unwrap(),
                )),
        );
    }
    units.push(
        unit(session, 1_002, SemanticRole::Link, "Target report")
            .with_affordance(ActionKind::Follow)
            .with_destination(Property::Known(
                AbsoluteUrl::new("https://example.test/report").unwrap(),
            )),
    );
    let subject = observation(session, units);
    let result = RelevanceSelector::default()
        .select(
            &subject,
            &budgeted(session, "target", 3),
            &FixedFragmentTokenizer::new(1),
            &SelectionContext::default(),
        )
        .unwrap();

    assert_eq!(
        selected_references(result.observation()),
        vec![reference(session, 1), reference(session, 1_002)]
    );
    assert_eq!(result.diagnostics().omitted_irrelevant_units(), 1_000);
    assert!(result.diagnostics().projection_bytes() > 0);
    assert!(
        result
            .diagnostics()
            .trace()
            .iter()
            .filter(|trace| trace.reference().sequence() <= 1_001)
            .skip(1)
            .all(|trace| trace.signals().repeated_navigation())
    );
}

#[test]
fn no_budget_preserves_full_state_and_budget_property_holds_for_nonessential_units() {
    let session = SessionId::new(37).unwrap();
    let mut units = vec![page(session)];
    for sequence in 2..=12 {
        units.push(
            unit(
                session,
                sequence,
                SemanticRole::Button,
                &format!("Action {sequence}"),
            )
            .with_affordance(ActionKind::Press),
        );
    }
    let subject = observation(session, units);
    let tokenizer = FixedFragmentTokenizer::new(1);
    let full = RelevanceSelector::default()
        .select(
            &subject,
            &ObservationRequest::new(session),
            &tokenizer,
            &SelectionContext::default(),
        )
        .unwrap();
    assert_eq!(full.observation().units().len(), subject.units().len());
    assert_eq!(
        full.observation()
            .omissions()
            .count(OmissionCategory::Budget),
        0
    );

    for budget in 2..=12 {
        let result = RelevanceSelector::default()
            .select(
                &subject,
                &budgeted(session, "action", budget),
                &tokenizer,
                &SelectionContext::default(),
            )
            .unwrap();
        assert_eq!(result.diagnostics().budget_overshoot_tokens(), 0);
        assert!(result.diagnostics().projected_tokens() <= budget);
        assert!(
            result
                .observation()
                .units()
                .iter()
                .all(|unit| subject.units().contains(unit))
        );
    }
}

#[test]
fn changed_reference_validation_and_configurable_weights_are_observable() {
    let session = SessionId::new(38).unwrap();
    let button = unit(session, 2, SemanticRole::Button, "Run").with_affordance(ActionKind::Press);
    let subject = observation(session, vec![page(session), button]);
    let missing = SelectionContext::default().with_changed(reference(session, 99));
    assert!(
        RelevanceSelector::default()
            .select(
                &subject,
                &ObservationRequest::new(session),
                &FixedFragmentTokenizer::new(1),
                &missing,
            )
            .is_err()
    );

    let mut weights = RankingConfig::default().weights();
    weights.interactive = 9_999;
    let selector = RelevanceSelector::new(
        RankingConfig::new("custom-ranking")
            .unwrap()
            .with_weights(weights),
    );
    let result = selector
        .select(
            &subject,
            &ObservationRequest::new(session),
            &FixedFragmentTokenizer::new(1),
            &SelectionContext::default(),
        )
        .unwrap();
    let button_trace = result
        .diagnostics()
        .trace()
        .iter()
        .find(|trace| trace.reference() == reference(session, 2))
        .unwrap();
    assert!(button_trace.score() >= 9_999);
    assert_eq!(result.diagnostics().ranking_version(), "custom-ranking");

    let selected_input = RelevanceSelector::default()
        .select(
            &subject,
            &budgeted(session, "unrelated", 2),
            &FixedFragmentTokenizer::new(1),
            &SelectionContext::default(),
        )
        .unwrap()
        .into_observation();
    let rejected = RelevanceSelector::default().select(
        &selected_input,
        &budgeted(session, "run", 2),
        &FixedFragmentTokenizer::new(1),
        &SelectionContext::default(),
    );
    assert!(rejected.is_err());
    assert!(
        RelevanceSelector::default()
            .select(
                &subject,
                &ObservationRequest::new(session),
                &FixedFragmentTokenizer::new(0),
                &SelectionContext::default(),
            )
            .is_err()
    );
}
