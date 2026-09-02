use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use mawr_core::{
    ElementRef, Measurement, MeasurementKind, MeasurementSource, NonEmptyText, Observation,
    ObservationRequest, OmissionCategory, OmissionSummary, OperationFailure, Property,
    RelationshipKind, SemanticRole, SemanticUnit, SemanticValue,
};

use crate::projection::{envelope_projection, unit_projection};
use crate::{RankingConfig, TokenCountQuality, TokenCounter, TokenizerMetadata};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectionContext {
    changed: BTreeSet<ElementRef>,
}

impl SelectionContext {
    #[must_use]
    pub fn with_changed(mut self, reference: ElementRef) -> Self {
        self.changed.insert(reference);
        self
    }

    pub fn changed(&self) -> impl Iterator<Item = ElementRef> + '_ {
        self.changed.iter().copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScoreSignals {
    name_overlap: u32,
    description_overlap: u32,
    value_overlap: u32,
    interactive: bool,
    structural: bool,
    alert: bool,
    invalid: bool,
    changed: bool,
    context: bool,
    boilerplate: bool,
    repeated_navigation: bool,
}

impl ScoreSignals {
    #[must_use]
    pub const fn name_overlap(self) -> u32 {
        self.name_overlap
    }

    #[must_use]
    pub const fn description_overlap(self) -> u32 {
        self.description_overlap
    }

    #[must_use]
    pub const fn value_overlap(self) -> u32 {
        self.value_overlap
    }

    #[must_use]
    pub const fn interactive(self) -> bool {
        self.interactive
    }

    #[must_use]
    pub const fn structural(self) -> bool {
        self.structural
    }

    #[must_use]
    pub const fn alert(self) -> bool {
        self.alert
    }

    #[must_use]
    pub const fn invalid(self) -> bool {
        self.invalid
    }

    #[must_use]
    pub const fn changed(self) -> bool {
        self.changed
    }

    #[must_use]
    pub const fn context(self) -> bool {
        self.context
    }

    #[must_use]
    pub const fn boilerplate(self) -> bool {
        self.boilerplate
    }

    #[must_use]
    pub const fn repeated_navigation(self) -> bool {
        self.repeated_navigation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitSelectionTrace {
    reference: ElementRef,
    score: i64,
    projected_tokens: u64,
    essential: bool,
    selected: bool,
    signals: ScoreSignals,
}

impl UnitSelectionTrace {
    #[must_use]
    pub const fn reference(&self) -> ElementRef {
        self.reference
    }

    #[must_use]
    pub const fn score(&self) -> i64 {
        self.score
    }

    #[must_use]
    pub const fn projected_tokens(&self) -> u64 {
        self.projected_tokens
    }

    #[must_use]
    pub const fn essential(&self) -> bool {
        self.essential
    }

    #[must_use]
    pub const fn selected(&self) -> bool {
        self.selected
    }

    #[must_use]
    pub const fn signals(&self) -> ScoreSignals {
        self.signals
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionDiagnostics {
    ranking_version: String,
    tokenizer: TokenizerMetadata,
    requested_budget: Option<u64>,
    reserved_tokens: u64,
    envelope_tokens: u64,
    projected_tokens: u64,
    budget_overshoot_tokens: u64,
    input_units: usize,
    selected_units: usize,
    omitted_budget_units: u64,
    omitted_irrelevant_units: u64,
    projection_bytes: u64,
    selection_latency_micros: u64,
    trace: Vec<UnitSelectionTrace>,
}

impl SelectionDiagnostics {
    #[must_use]
    pub fn ranking_version(&self) -> &str {
        &self.ranking_version
    }

    #[must_use]
    pub const fn tokenizer(&self) -> &TokenizerMetadata {
        &self.tokenizer
    }

    #[must_use]
    pub const fn requested_budget(&self) -> Option<u64> {
        self.requested_budget
    }

    #[must_use]
    pub const fn reserved_tokens(&self) -> u64 {
        self.reserved_tokens
    }

    #[must_use]
    pub const fn envelope_tokens(&self) -> u64 {
        self.envelope_tokens
    }

    #[must_use]
    pub const fn projected_tokens(&self) -> u64 {
        self.projected_tokens
    }

    #[must_use]
    pub const fn budget_overshoot_tokens(&self) -> u64 {
        self.budget_overshoot_tokens
    }

    #[must_use]
    pub const fn input_units(&self) -> usize {
        self.input_units
    }

    #[must_use]
    pub const fn selected_units(&self) -> usize {
        self.selected_units
    }

    #[must_use]
    pub const fn omitted_budget_units(&self) -> u64 {
        self.omitted_budget_units
    }

    #[must_use]
    pub const fn omitted_irrelevant_units(&self) -> u64 {
        self.omitted_irrelevant_units
    }

    #[must_use]
    pub const fn projection_bytes(&self) -> u64 {
        self.projection_bytes
    }

    #[must_use]
    pub const fn selection_latency_micros(&self) -> u64 {
        self.selection_latency_micros
    }

    #[must_use]
    pub fn trace(&self) -> &[UnitSelectionTrace] {
        &self.trace
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedObservation {
    observation: Observation,
    diagnostics: SelectionDiagnostics,
}

impl RankedObservation {
    #[must_use]
    pub const fn observation(&self) -> &Observation {
        &self.observation
    }

    #[must_use]
    pub const fn diagnostics(&self) -> &SelectionDiagnostics {
        &self.diagnostics
    }

    #[must_use]
    pub fn into_observation(self) -> Observation {
        self.observation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelevanceSelector {
    config: RankingConfig,
}

impl RelevanceSelector {
    #[must_use]
    pub const fn new(config: RankingConfig) -> Self {
        Self { config }
    }

    pub fn select<T: TokenCounter>(
        &self,
        observation: &Observation,
        request: &ObservationRequest,
        tokenizer: &T,
        context: &SelectionContext,
    ) -> Result<RankedObservation, OperationFailure> {
        if request.session() != observation.state().session() {
            return Err(OperationFailure::session_mismatch(
                "selection_request_session",
                observation.state().session(),
                request.session(),
            ));
        }
        if observation.omissions().iter().any(|(_, count)| count > 0) {
            return Err(invariant_failure("selection_requires_full_observation"));
        }
        if observation
            .units()
            .iter()
            .filter(|unit| unit.role() == SemanticRole::Page)
            .count()
            != 1
        {
            return Err(invariant_failure("selection_requires_one_page"));
        }
        let reference_to_index = observation
            .units()
            .iter()
            .enumerate()
            .map(|(index, unit)| (unit.reference(), index))
            .collect::<BTreeMap<_, _>>();
        for reference in context.changed() {
            if reference.session() != observation.state().session() {
                return Err(OperationFailure::session_mismatch(
                    "changed_reference_session",
                    observation.state().session(),
                    reference.session(),
                ));
            }
            if !reference_to_index.contains_key(&reference) {
                return Err(OperationFailure::MissingReference { reference });
            }
        }

        let started = Instant::now();
        let goal = normalized_terms(request.goal().unwrap_or_default());
        let duplicate_text = duplicate_text_counts(observation.units());
        let duplicate_navigation = duplicate_navigation_counts(observation.units());
        let weights = self.config.weights();
        let mut ranked = observation
            .units()
            .iter()
            .map(|unit| {
                score_unit(
                    unit,
                    &goal,
                    &duplicate_text,
                    &duplicate_navigation,
                    context,
                    weights,
                    tokenizer,
                )
            })
            .collect::<Vec<_>>();
        let dependencies = (0..ranked.len())
            .map(|index| dependency_closure(index, observation.units(), &reference_to_index))
            .collect::<Vec<_>>();

        let contextual = ranked
            .iter()
            .enumerate()
            .filter(|(_, candidate)| {
                candidate.essential || candidate.score >= self.config.minimum_score()
            })
            .flat_map(|(index, _)| dependencies[index].iter().copied())
            .collect::<BTreeSet<_>>();
        for index in contextual {
            ranked[index].signals.context = true;
            ranked[index].score = ranked[index].score.saturating_add(weights.context);
        }

        let envelope = envelope_projection(observation);
        let envelope_tokens = tokenizer.count_tokens(&envelope);
        if envelope_tokens == 0
            || ranked
                .iter()
                .any(|candidate| candidate.projected_tokens == 0)
        {
            return Err(invariant_failure("tokenizer_zero_for_nonempty_fragment"));
        }
        let requested_budget = request.max_tokens().map(|budget| budget.get());
        let mut selected = BTreeSet::new();
        let mut selected_unit_tokens = 0_u64;
        let mut budget_blocked = BTreeSet::new();

        if let Some(budget) = requested_budget {
            for index in 0..ranked.len() {
                if ranked[index].essential {
                    add_bundle(
                        index,
                        &dependencies,
                        &ranked,
                        &mut selected,
                        &mut selected_unit_tokens,
                    );
                }
            }

            let mut order = (0..ranked.len()).collect::<Vec<_>>();
            order.sort_unstable_by(|left, right| {
                ranked[*right]
                    .score
                    .cmp(&ranked[*left].score)
                    .then_with(|| ranked[*left].reference.cmp(&ranked[*right].reference))
            });
            for index in order {
                if selected.contains(&index) || ranked[index].score < self.config.minimum_score() {
                    continue;
                }
                let bundle = missing_bundle(index, &dependencies, &selected);
                let bundle_tokens = bundle
                    .iter()
                    .map(|unit_index| ranked[*unit_index].projected_tokens)
                    .fold(0_u64, u64::saturating_add);
                let projected_total = self
                    .config
                    .reserved_tokens()
                    .saturating_add(envelope_tokens)
                    .saturating_add(selected_unit_tokens)
                    .saturating_add(bundle_tokens);
                if projected_total <= budget {
                    for unit_index in bundle {
                        if selected.insert(unit_index) {
                            selected_unit_tokens = selected_unit_tokens
                                .saturating_add(ranked[unit_index].projected_tokens);
                        }
                    }
                } else {
                    budget_blocked.extend(bundle);
                }
            }
        } else {
            selected.extend(0..ranked.len());
            selected_unit_tokens = ranked
                .iter()
                .map(|candidate| candidate.projected_tokens)
                .fold(0_u64, u64::saturating_add);
        }

        let mut omissions = OmissionSummary::new();
        let mut omitted_budget_units = 0_u64;
        let mut omitted_irrelevant_units = 0_u64;
        for (index, candidate) in ranked.iter_mut().enumerate() {
            candidate.selected = selected.contains(&index);
            if candidate.selected {
                continue;
            }
            if budget_blocked.contains(&index) || candidate.score >= self.config.minimum_score() {
                omitted_budget_units = omitted_budget_units.saturating_add(1);
            } else {
                omitted_irrelevant_units = omitted_irrelevant_units.saturating_add(1);
            }
        }
        if omitted_budget_units > 0 {
            omissions = omissions
                .record(OmissionCategory::Budget, omitted_budget_units)
                .map_err(OperationFailure::InvalidInput)?;
        }
        if omitted_irrelevant_units > 0 {
            omissions = omissions
                .record(OmissionCategory::Irrelevant, omitted_irrelevant_units)
                .map_err(OperationFailure::InvalidInput)?;
        }

        let selected_references = selected
            .iter()
            .map(|index| ranked[*index].reference)
            .collect::<Vec<_>>();
        let projected_tokens = envelope_tokens.saturating_add(selected_unit_tokens);
        let observation_token_measurement = match tokenizer.metadata().quality() {
            TokenCountQuality::Exact => Measurement::Exact {
                value: projected_tokens,
                source: MeasurementSource::LocalTokenizer,
            },
            TokenCountQuality::Estimated => Measurement::Estimated {
                value: projected_tokens,
                method: NonEmptyText::new(
                    format!(
                        "{}@{} additive fragments",
                        tokenizer.metadata().name(),
                        tokenizer.metadata().version()
                    ),
                    "observation_token_method",
                )
                .expect("validated tokenizer metadata fits the measurement method"),
            },
        };
        let measurements = observation.measurements().clone().with(
            MeasurementKind::ObservationTokens,
            observation_token_measurement,
        );
        let observation = observation
            .clone()
            .with_selected_units(selected_references, omissions)
            .map_err(OperationFailure::InvalidInput)?
            .with_measurements(measurements);
        let consumed_with_reserve = self
            .config
            .reserved_tokens()
            .saturating_add(projected_tokens);
        let budget_overshoot_tokens = requested_budget
            .map(|budget| consumed_with_reserve.saturating_sub(budget))
            .unwrap_or(0);
        let projection_bytes = u64::try_from(envelope.len())
            .unwrap_or(u64::MAX)
            .saturating_add(
                selected
                    .iter()
                    .map(|index| ranked[*index].projection_bytes)
                    .fold(0_u64, u64::saturating_add),
            );
        let selection_latency_micros =
            u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        let diagnostics = SelectionDiagnostics {
            ranking_version: self.config.version().to_owned(),
            tokenizer: tokenizer.metadata().clone(),
            requested_budget,
            reserved_tokens: self.config.reserved_tokens(),
            envelope_tokens,
            projected_tokens,
            budget_overshoot_tokens,
            input_units: ranked.len(),
            selected_units: observation.units().len(),
            omitted_budget_units,
            omitted_irrelevant_units,
            projection_bytes,
            selection_latency_micros,
            trace: ranked.into_iter().map(RankedUnit::into_trace).collect(),
        };
        Ok(RankedObservation {
            observation,
            diagnostics,
        })
    }
}

impl Default for RelevanceSelector {
    fn default() -> Self {
        Self::new(RankingConfig::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RankedUnit {
    reference: ElementRef,
    score: i64,
    projected_tokens: u64,
    projection_bytes: u64,
    essential: bool,
    selected: bool,
    signals: ScoreSignals,
}

impl RankedUnit {
    fn into_trace(self) -> UnitSelectionTrace {
        UnitSelectionTrace {
            reference: self.reference,
            score: self.score,
            projected_tokens: self.projected_tokens,
            essential: self.essential,
            selected: self.selected,
            signals: self.signals,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn score_unit<T: TokenCounter>(
    unit: &SemanticUnit,
    goal: &BTreeSet<String>,
    duplicate_text: &BTreeMap<String, usize>,
    duplicate_navigation: &BTreeMap<String, usize>,
    context: &SelectionContext,
    weights: crate::RankingWeights,
    tokenizer: &T,
) -> RankedUnit {
    let name_overlap = property_overlap(unit.name(), goal);
    let description_overlap = property_overlap(unit.description(), goal);
    let value_overlap = match unit.value() {
        SemanticValue::Text(value) => term_overlap(value.as_str(), goal),
        SemanticValue::Unknown(reason) => term_overlap(reason.as_str(), goal),
        SemanticValue::Absent | SemanticValue::Redacted => 0,
    };
    let repeated_text = unit_text_key(unit)
        .and_then(|key| duplicate_text.get(&key))
        .is_some_and(|count| *count >= 3);
    let repeated_navigation = navigation_key(unit)
        .and_then(|key| duplicate_navigation.get(&key))
        .is_some_and(|count| *count >= 3);
    let signals = ScoreSignals {
        name_overlap,
        description_overlap,
        value_overlap,
        interactive: unit.affordances().iter().next().is_some(),
        structural: matches!(
            unit.role(),
            SemanticRole::Page
                | SemanticRole::Region
                | SemanticRole::Heading
                | SemanticRole::Form
                | SemanticRole::Table
                | SemanticRole::Row
                | SemanticRole::List
        ),
        alert: unit.role() == SemanticRole::Alert,
        invalid: matches!(unit.state().invalid(), Property::Known(true)),
        changed: context.changed.contains(&unit.reference()),
        context: false,
        boilerplate: repeated_text
            && matches!(
                unit.role(),
                SemanticRole::Text | SemanticRole::Link | SemanticRole::ListItem
            ),
        repeated_navigation,
    };
    let mut score = 0_i64;
    score =
        score.saturating_add(i64::from(name_overlap).saturating_mul(weights.goal_name_per_term));
    score = score.saturating_add(
        i64::from(description_overlap).saturating_mul(weights.goal_description_per_term),
    );
    score =
        score.saturating_add(i64::from(value_overlap).saturating_mul(weights.goal_value_per_term));
    score = score.saturating_add(bool_weight(signals.interactive, weights.interactive));
    score = score.saturating_add(bool_weight(signals.structural, weights.structural));
    score = score.saturating_add(bool_weight(signals.alert, weights.alert));
    score = score.saturating_add(bool_weight(signals.invalid, weights.invalid));
    score = score.saturating_add(bool_weight(signals.changed, weights.changed));
    score = score.saturating_sub(bool_weight(
        signals.boilerplate,
        weights.boilerplate_penalty,
    ));
    score = score.saturating_sub(bool_weight(
        signals.repeated_navigation,
        weights.repeated_navigation_penalty,
    ));
    let projection = unit_projection(unit);
    RankedUnit {
        reference: unit.reference(),
        score,
        projected_tokens: tokenizer.count_tokens(&projection),
        projection_bytes: u64::try_from(projection.len()).unwrap_or(u64::MAX),
        essential: signals.alert || signals.invalid || unit.role() == SemanticRole::Page,
        selected: false,
        signals,
    }
}

fn bool_weight(condition: bool, weight: i64) -> i64 {
    if condition { weight } else { 0 }
}

fn invariant_failure(code: &'static str) -> OperationFailure {
    OperationFailure::InvariantViolation {
        code: NonEmptyText::new(code, "relevance_invariant")
            .expect("static relevance invariant code is valid"),
    }
}

fn property_overlap<const MAX: usize>(
    property: &Property<mawr_core::BoundedText<MAX>>,
    goal: &BTreeSet<String>,
) -> u32 {
    match property {
        Property::Known(value) => term_overlap(value.as_str(), goal),
        Property::NotApplicable | Property::Unknown(_) => 0,
    }
}

fn term_overlap(value: &str, goal: &BTreeSet<String>) -> u32 {
    u32::try_from(normalized_terms(value).intersection(goal).count()).unwrap_or(u32::MAX)
}

fn normalized_terms(value: &str) -> BTreeSet<String> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| term.to_lowercase())
        .collect()
}

fn unit_text_key(unit: &SemanticUnit) -> Option<String> {
    match unit.name() {
        Property::Known(name) if !name.as_str().trim().is_empty() => {
            Some(name.as_str().trim().to_lowercase())
        }
        _ => match unit.value() {
            SemanticValue::Text(value) if !value.as_str().trim().is_empty() => {
                Some(value.as_str().trim().to_lowercase())
            }
            _ => None,
        },
    }
}

fn duplicate_text_counts(units: &[SemanticUnit]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for key in units.iter().filter_map(unit_text_key) {
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

fn navigation_key(unit: &SemanticUnit) -> Option<String> {
    if unit.role() != SemanticRole::Link {
        return None;
    }
    let name = unit_text_key(unit)?;
    let destination = match unit.destination() {
        Property::Known(destination) => destination.as_str(),
        Property::NotApplicable | Property::Unknown(_) => "",
    };
    Some(format!("{name}\u{1f}{destination}"))
}

fn duplicate_navigation_counts(units: &[SemanticUnit]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for key in units.iter().filter_map(navigation_key) {
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

fn dependency_closure(
    index: usize,
    units: &[SemanticUnit],
    reference_to_index: &BTreeMap<ElementRef, usize>,
) -> BTreeSet<usize> {
    let mut closure = BTreeSet::new();
    let mut pending = vec![index];
    while let Some(current) = pending.pop() {
        let unit = &units[current];
        if let Some(parent) = unit
            .parent()
            .and_then(|value| reference_to_index.get(&value))
            && closure.insert(*parent)
        {
            pending.push(*parent);
        }
        for relationship in unit.relationships() {
            if matches!(
                relationship.kind(),
                RelationshipKind::LabelledBy
                    | RelationshipKind::DescribedBy
                    | RelationshipKind::OwnedBy
                    | RelationshipKind::OptionOf
                    | RelationshipKind::RowOf
                    | RelationshipKind::CellOf
                    | RelationshipKind::ListItemOf
            ) && let Some(target) = reference_to_index.get(&relationship.target())
                && closure.insert(*target)
            {
                pending.push(*target);
            }
        }
    }
    closure.remove(&index);
    closure
}

fn missing_bundle(
    index: usize,
    dependencies: &[BTreeSet<usize>],
    selected: &BTreeSet<usize>,
) -> BTreeSet<usize> {
    dependencies[index]
        .iter()
        .copied()
        .chain(std::iter::once(index))
        .filter(|candidate| !selected.contains(candidate))
        .collect()
}

fn add_bundle(
    index: usize,
    dependencies: &[BTreeSet<usize>],
    ranked: &[RankedUnit],
    selected: &mut BTreeSet<usize>,
    selected_tokens: &mut u64,
) {
    for unit_index in dependencies[index]
        .iter()
        .copied()
        .chain(std::iter::once(index))
    {
        if selected.insert(unit_index) {
            *selected_tokens = selected_tokens.saturating_add(ranked[unit_index].projected_tokens);
        }
    }
}
