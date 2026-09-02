use std::num::NonZeroU64;
use std::time::Instant;

use mawr_core::{
    Action, ActionKind, ActionRequest, AuthorizationReason, Capability, CapabilityConstraint,
    CapabilityConstraints, CapabilityReport, CapabilityStatus, ElementRef, EngineFailureKind,
    NonEmptyText, OperationFailure, OperationKind, PressCommand, Property, ResourceKind,
    SemanticRole, SensitiveText, StateId, TransitionCause, UnsupportedReason, ValidationIssue,
};
use mawr_native_static::{
    CancellationToken, FormField, FormMethod, FormSubmission, NavigationRequest, StaticSession,
};
use mawr_semantic_html::{
    HtmlSemanticExtractor, MAX_CONTROL_VALUE_BYTES, SemanticDocument, SourceNodeId,
    StaticFormEncoding, StaticFormMethod, StaticInteractionKind,
};
use mawr_state::{SemanticStateStore, StoredSemanticUnit};

use crate::model::{
    ActionAuthorizationContext, ActionAuthorizer, ActionDiagnostics, ActionExecutionFailure,
    ActionOutcome, AuthorizationDecision, NetworkEvidence, SideEffectStatus,
};

enum Effect {
    Fill {
        source: SourceNodeId,
        value: SensitiveText<MAX_CONTROL_VALUE_BYTES>,
    },
    Checked {
        source: SourceNodeId,
        checked: bool,
    },
    Select {
        select: SourceNodeId,
        option: SourceNodeId,
    },
    Network(NavigationRequest),
}

struct Prepared {
    effective: ActionKind,
    authorization: ActionAuthorizationContext,
    effect: Effect,
}

pub struct StaticActionExecutor<A> {
    session: StaticSession,
    extractor: HtmlSemanticExtractor,
    store: SemanticStateStore,
    authorizer: A,
    capabilities: CapabilityReport,
}

impl<A: ActionAuthorizer> StaticActionExecutor<A> {
    pub fn new(
        session: StaticSession,
        extractor: HtmlSemanticExtractor,
        store: SemanticStateStore,
        authorizer: A,
    ) -> Result<Self, OperationFailure> {
        if session.id() != store.session() {
            return Err(OperationFailure::session_mismatch(
                "action_executor_session",
                session.id(),
                store.session(),
            ));
        }
        if session.engine_identity() != store.engine() {
            return Err(OperationFailure::EngineFailure {
                engine: session.engine_identity().clone(),
                kind: EngineFailureKind::CapabilityMismatch,
            });
        }
        let static_only = || {
            CapabilityStatus::Limited(CapabilityConstraints::new(CapabilityConstraint::Other(
                NonEmptyText::new("native-static-only", "capability_constraint")
                    .expect("static capability constraint is valid"),
            )))
        };
        let capabilities = session
            .capabilities()
            .clone()
            .with(Capability::HtmlParsing, CapabilityStatus::Supported)
            .with(Capability::SemanticContent, CapabilityStatus::Supported)
            .with(Capability::TextInput, static_only())
            .with(Capability::Checkbox, static_only())
            .with(Capability::Radio, static_only())
            .with(Capability::Select, static_only())
            .with(Capability::Button, static_only());
        Ok(Self {
            session,
            extractor,
            store,
            authorizer,
            capabilities,
        })
    }

    #[must_use]
    pub const fn capabilities(&self) -> &CapabilityReport {
        &self.capabilities
    }
    #[must_use]
    pub const fn store(&self) -> &SemanticStateStore {
        &self.store
    }
    #[must_use]
    pub const fn session(&self) -> &StaticSession {
        &self.session
    }
    #[must_use]
    pub fn into_parts(self) -> (StaticSession, SemanticStateStore, A) {
        (self.session, self.store, self.authorizer)
    }

    pub async fn execute(
        &mut self,
        request: ActionRequest,
        cancellation: &CancellationToken,
    ) -> Result<ActionOutcome, ActionExecutionFailure> {
        let started = Instant::now();
        let requested = request.action().kind();
        let prepared = self
            .prepare(
                request.expected_state(),
                request.action().clone(),
                requested,
            )
            .map_err(ActionExecutionFailure::preflight)?;
        if let AuthorizationDecision::Deny(reason) =
            self.authorizer.authorize(&prepared.authorization)
        {
            return Err(ActionExecutionFailure::preflight(
                OperationFailure::AuthorizationDenied {
                    operation: prepared.authorization.operation(),
                    reason,
                },
            ));
        }

        let (update, network) = match prepared.effect {
            Effect::Fill { source, value } => {
                let mut document = self
                    .current_document(request.expected_state())
                    .map_err(ActionExecutionFailure::preflight)?;
                if !document.fill_static_control(source, value) {
                    return Err(ActionExecutionFailure::preflight(invariant(
                        "validated_fill_failed",
                    )));
                }
                let update = self
                    .store
                    .update(document, TransitionCause::Action(requested))
                    .map_err(ActionExecutionFailure::preflight)?;
                (update, None)
            }
            Effect::Checked { source, checked } => {
                let mut document = self
                    .current_document(request.expected_state())
                    .map_err(ActionExecutionFailure::preflight)?;
                if !document.set_static_checked(source, checked) {
                    return Err(ActionExecutionFailure::preflight(invariant(
                        "validated_check_failed",
                    )));
                }
                let update = self
                    .store
                    .update(document, TransitionCause::Action(requested))
                    .map_err(ActionExecutionFailure::preflight)?;
                (update, None)
            }
            Effect::Select { select, option } => {
                let mut document = self
                    .current_document(request.expected_state())
                    .map_err(ActionExecutionFailure::preflight)?;
                if !document.select_static_option(select, option) {
                    return Err(ActionExecutionFailure::preflight(invariant(
                        "validated_select_failed",
                    )));
                }
                let update = self
                    .store
                    .update(document, TransitionCause::Action(requested))
                    .map_err(ActionExecutionFailure::preflight)?;
                (update, None)
            }
            Effect::Network(navigation) => {
                let method = navigation.method();
                let document = self
                    .session
                    .navigate(navigation, cancellation)
                    .await
                    .map_err(|failure| {
                        ActionExecutionFailure::new(failure, SideEffectStatus::Requested, None)
                    })?;
                let evidence = NetworkEvidence::from_document(method, &document);
                let semantic = self.extractor.extract(&document).map_err(|failure| {
                    ActionExecutionFailure::new(
                        failure,
                        SideEffectStatus::NetworkCompleted,
                        Some(evidence.clone()),
                    )
                })?;
                let update = self
                    .store
                    .update(semantic, TransitionCause::Action(requested))
                    .map_err(|failure| {
                        ActionExecutionFailure::new(
                            failure,
                            SideEffectStatus::NetworkCompleted,
                            Some(evidence.clone()),
                        )
                    })?;
                (update, Some(evidence))
            }
        };
        let latency = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        Ok(ActionOutcome::new(
            requested,
            prepared.effective,
            update,
            network,
            ActionDiagnostics::new(latency),
        ))
    }

    fn current_document(&self, expected: StateId) -> Result<SemanticDocument, OperationFailure> {
        self.ensure_current(expected)?;
        Ok(self
            .store
            .current()
            .expect("current state was verified")
            .document()
            .clone())
    }

    fn prepare(
        &self,
        expected: StateId,
        action: Action,
        requested: ActionKind,
    ) -> Result<Prepared, OperationFailure> {
        self.ensure_current(expected)?;
        let pressed_target = match &action {
            Action::Press { target, .. } => *target,
            _ => None,
        };
        let (action, effective) = self.normalize_press(expected, action)?;
        match action {
            Action::Navigate(destination) => {
                self.require_capability(Capability::Navigation)?;
                let navigation = NavigationRequest::get(destination.clone());
                Ok(self.network_prepared(
                    requested,
                    effective,
                    expected,
                    None,
                    destination,
                    navigation,
                ))
            }
            Action::Follow(target) => {
                self.require_capability(Capability::Navigation)?;
                let unit = self.resolve(expected, target)?;
                self.require_affordance(unit, ActionKind::Follow, Capability::Navigation)?;
                if unit.semantic().role() != SemanticRole::Link
                    || unit.semantic().interaction().kind() != StaticInteractionKind::Link
                {
                    return Err(self.unsupported(Capability::Navigation));
                }
                if unit.semantic().interaction().download() {
                    return Err(OperationFailure::AuthorizationDenied {
                        operation: OperationKind::Download,
                        reason: AuthorizationReason::ConfirmationRequired,
                    });
                }
                let destination = known_destination(unit)?.clone();
                let navigation = NavigationRequest::get(destination.clone());
                Ok(self.network_prepared(
                    requested,
                    effective,
                    expected,
                    pressed_target.or(Some(target)),
                    destination,
                    navigation,
                ))
            }
            Action::Fill { target, value } => {
                self.require_capability(Capability::TextInput)?;
                let unit = self.resolve(expected, target)?;
                self.require_affordance(unit, ActionKind::Fill, Capability::TextInput)?;
                if unit.semantic().interaction().kind() != StaticInteractionKind::TextControl {
                    return Err(self.unsupported(Capability::TextInput));
                }
                Ok(self.local_prepared(
                    requested,
                    effective,
                    expected,
                    pressed_target.or(Some(target)),
                    Effect::Fill {
                        source: unit.semantic().source(),
                        value,
                    },
                ))
            }
            Action::Check(target) | Action::Uncheck(target) => {
                let checked = effective == ActionKind::Check;
                let unit = self.resolve(expected, target)?;
                let kind = unit.semantic().interaction().kind();
                let capability = if kind == StaticInteractionKind::Radio {
                    Capability::Radio
                } else {
                    Capability::Checkbox
                };
                self.require_capability(capability)?;
                self.require_affordance(unit, effective, capability)?;
                if !matches!(
                    kind,
                    StaticInteractionKind::Checkbox | StaticInteractionKind::Radio
                ) || kind == StaticInteractionKind::Radio && !checked
                {
                    return Err(self.unsupported(capability));
                }
                Ok(self.local_prepared(
                    requested,
                    effective,
                    expected,
                    pressed_target.or(Some(target)),
                    Effect::Checked {
                        source: unit.semantic().source(),
                        checked,
                    },
                ))
            }
            Action::Select { target, option } => {
                self.require_capability(Capability::Select)?;
                let select = self.resolve(expected, target)?;
                let option_unit = self.resolve(expected, option)?;
                self.require_affordance(select, ActionKind::Select, Capability::Select)?;
                self.require_affordance(option_unit, ActionKind::Select, Capability::Select)?;
                if select.semantic().interaction().kind() != StaticInteractionKind::Select
                    || option_unit.semantic().interaction().kind() != StaticInteractionKind::Option
                    || option_unit.semantic().interaction().owner()
                        != Some(select.semantic().source())
                {
                    return Err(OperationFailure::invalid_input(
                        "select_option",
                        ValidationIssue::InvalidFormat,
                    ));
                }
                Ok(self.local_prepared(
                    requested,
                    effective,
                    expected,
                    pressed_target.or(Some(target)),
                    Effect::Select {
                        select: select.semantic().source(),
                        option: option_unit.semantic().source(),
                    },
                ))
            }
            Action::Submit(target) => self.prepare_submit(
                expected,
                target,
                pressed_target.or(Some(target)),
                requested,
                effective,
            ),
            Action::Press { .. } => unreachable!("press actions are normalized before dispatch"),
        }
    }

    fn normalize_press(
        &self,
        expected: StateId,
        action: Action,
    ) -> Result<(Action, ActionKind), OperationFailure> {
        let Action::Press { target, command } = action else {
            let kind = action.kind();
            return Ok((action, kind));
        };
        let Some(target) = target else {
            return Err(self.unsupported(Capability::KeyInput));
        };
        let unit = self.resolve(expected, target)?;
        let kind = unit.semantic().interaction().kind();
        let normalized = match (kind, command) {
            (StaticInteractionKind::Link, PressCommand::Enter) => Action::follow(target),
            (StaticInteractionKind::Checkbox, PressCommand::Space) => {
                if unit.semantic().state().checked() == &Property::Known(true) {
                    Action::uncheck(target)
                } else {
                    Action::check(target)
                }
            }
            (StaticInteractionKind::Radio, PressCommand::Space) => Action::check(target),
            (StaticInteractionKind::SubmitButton, PressCommand::Enter | PressCommand::Space) => {
                Action::submit(target)
            }
            (StaticInteractionKind::TextControl, PressCommand::Enter) => {
                let Some(form_source) = unit.semantic().interaction().owner() else {
                    return Err(self.unsupported(Capability::KeyInput));
                };
                let form = self
                    .store
                    .current()
                    .expect("current state was verified")
                    .units()
                    .iter()
                    .find(|candidate| candidate.semantic().source() == form_source)
                    .ok_or_else(|| invariant("missing_form_reference"))?;
                Action::submit(form.reference())
            }
            (StaticInteractionKind::Button, PressCommand::Enter | PressCommand::Space) => {
                return Err(self.unsupported(Capability::JavaScript));
            }
            (StaticInteractionKind::ImageButton, PressCommand::Enter | PressCommand::Space) => {
                return Err(self.unsupported(Capability::Geometry));
            }
            (StaticInteractionKind::ResetButton, PressCommand::Enter | PressCommand::Space) => {
                return Err(self.unsupported(Capability::Button));
            }
            _ => return Err(self.unsupported(Capability::KeyInput)),
        };
        let effective = normalized.kind();
        Ok((normalized, effective))
    }

    fn prepare_submit(
        &self,
        expected: StateId,
        target: ElementRef,
        authorization_target: Option<ElementRef>,
        requested: ActionKind,
        effective: ActionKind,
    ) -> Result<Prepared, OperationFailure> {
        let target_unit = self.resolve(expected, target)?;
        self.require_affordance(target_unit, ActionKind::Submit, Capability::Button)?;
        let interaction = target_unit.semantic().interaction();
        let form_source = match interaction.kind() {
            StaticInteractionKind::Form => target_unit.semantic().source(),
            StaticInteractionKind::SubmitButton => interaction
                .owner()
                .ok_or_else(|| invariant("submitter_without_form"))?,
            _ => return Err(self.unsupported(Capability::Button)),
        };
        let method = match interaction.method() {
            Some(StaticFormMethod::Get) => {
                self.require_capability(Capability::FormGet)?;
                FormMethod::Get
            }
            Some(StaticFormMethod::Post) => {
                self.require_capability(Capability::FormPost)?;
                FormMethod::Post
            }
            _ => return Err(self.unsupported(Capability::Button)),
        };
        if method == FormMethod::Post
            && interaction.encoding() != Some(StaticFormEncoding::UrlEncoded)
        {
            return Err(self.unsupported(match method {
                FormMethod::Get => Capability::FormGet,
                FormMethod::Post => Capability::FormPost,
            }));
        }
        let destination = known_destination(target_unit)?.clone();
        let submitter = (interaction.kind() == StaticInteractionKind::SubmitButton)
            .then(|| target_unit.semantic().source());
        let fields = self.form_fields(form_source, submitter, !interaction.no_validate())?;
        let submission =
            FormSubmission::new(fields).map_err(|_| OperationFailure::ResourceLimit {
                resource: ResourceKind::Actions,
                configured_limit: NonZeroU64::new(256).expect("form field limit is non-zero"),
            })?;
        let navigation = NavigationRequest::submit_form(destination.clone(), method, submission);
        Ok(self.network_prepared(
            requested,
            effective,
            expected,
            authorization_target,
            destination,
            navigation,
        ))
    }

    fn form_fields(
        &self,
        form_source: SourceNodeId,
        submitter: Option<SourceNodeId>,
        validate: bool,
    ) -> Result<Vec<FormField>, OperationFailure> {
        let state = self.store.current().expect("current state was verified");
        let mut fields = Vec::new();
        let mut hidden = state
            .document()
            .hidden_controls()
            .iter()
            .filter(|control| control.owner() == form_source)
            .peekable();
        for unit in state.units() {
            let semantic = unit.semantic();
            while hidden
                .peek()
                .is_some_and(|control| control.source() < semantic.source())
            {
                append_hidden(&mut fields, hidden.next().expect("peeked hidden control"))?;
            }
            let interaction = semantic.interaction();
            if interaction.owner() != Some(form_source)
                || semantic.state().disabled() == &Property::Known(true)
            {
                continue;
            }
            if !interaction.supported() {
                return Err(OperationFailure::invalid_input(
                    "form_control",
                    ValidationIssue::InvalidFormat,
                ));
            }
            if validate {
                self.validate_form_control(unit, form_source)?;
            }
            match interaction.kind() {
                StaticInteractionKind::TextControl => {
                    push_named_value(&mut fields, interaction)?;
                }
                StaticInteractionKind::Checkbox | StaticInteractionKind::Radio
                    if semantic.state().checked() == &Property::Known(true) =>
                {
                    push_named_value(&mut fields, interaction)?;
                }
                StaticInteractionKind::Select => {
                    let Some(name) = interaction.name() else {
                        continue;
                    };
                    for option in state.units().iter().filter(|option| {
                        option.semantic().interaction().kind() == StaticInteractionKind::Option
                            && option.semantic().interaction().owner() == Some(semantic.source())
                            && option.semantic().state().selected() == &Property::Known(true)
                            && option.semantic().state().disabled() != &Property::Known(true)
                    }) {
                        let value = option
                            .semantic()
                            .interaction()
                            .submission_value()
                            .ok_or_else(|| {
                                OperationFailure::invalid_input(
                                    "select_option_value",
                                    ValidationIssue::InvalidFormat,
                                )
                            })?;
                        fields.push(
                            FormField::new(name, value).map_err(OperationFailure::InvalidInput)?,
                        );
                    }
                }
                StaticInteractionKind::SubmitButton if submitter == Some(semantic.source()) => {
                    push_named_value(&mut fields, interaction)?;
                }
                StaticInteractionKind::FileControl if interaction.name().is_some() => {
                    return Err(OperationFailure::invalid_input(
                        "file_upload",
                        ValidationIssue::InvalidFormat,
                    ));
                }
                _ => {}
            }
            if fields.len() > 256 {
                return Err(OperationFailure::ResourceLimit {
                    resource: ResourceKind::Actions,
                    configured_limit: NonZeroU64::new(256).expect("form field limit is non-zero"),
                });
            }
        }
        for control in hidden {
            append_hidden(&mut fields, control)?;
        }
        Ok(fields)
    }

    fn validate_form_control(
        &self,
        unit: &StoredSemanticUnit,
        form_source: SourceNodeId,
    ) -> Result<(), OperationFailure> {
        let semantic = unit.semantic();
        if semantic.state().invalid() == &Property::Known(true) {
            return Err(OperationFailure::invalid_input(
                "form_control_validity",
                ValidationIssue::InvalidTransition,
            ));
        }
        if semantic.state().required() != &Property::Known(true) {
            return Ok(());
        }
        let interaction = semantic.interaction();
        let valid = match interaction.kind() {
            StaticInteractionKind::TextControl => interaction
                .submission_value()
                .is_some_and(|value| !value.is_empty()),
            StaticInteractionKind::Checkbox => semantic.state().checked() == &Property::Known(true),
            StaticInteractionKind::Select => self
                .store
                .current()
                .expect("current state was verified")
                .units()
                .iter()
                .any(|option| {
                    option.semantic().interaction().kind() == StaticInteractionKind::Option
                        && option.semantic().interaction().owner() == Some(semantic.source())
                        && option.semantic().state().selected() == &Property::Known(true)
                }),
            StaticInteractionKind::Radio => self
                .store
                .current()
                .expect("current state was verified")
                .units()
                .iter()
                .any(|candidate| {
                    candidate.semantic().interaction().kind() == StaticInteractionKind::Radio
                        && candidate.semantic().interaction().owner() == Some(form_source)
                        && candidate.semantic().interaction().name() == interaction.name()
                        && candidate.semantic().state().checked() == &Property::Known(true)
                }),
            _ => true,
        };
        if valid {
            Ok(())
        } else {
            Err(OperationFailure::invalid_input(
                "required_form_control",
                ValidationIssue::InvalidTransition,
            ))
        }
    }

    fn local_prepared(
        &self,
        requested: ActionKind,
        effective: ActionKind,
        expected: StateId,
        target: Option<ElementRef>,
        effect: Effect,
    ) -> Prepared {
        Prepared {
            effective,
            authorization: ActionAuthorizationContext::new(
                OperationKind::Act(requested),
                requested,
                effective,
                expected,
                target,
                None,
                None,
            ),
            effect,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn network_prepared(
        &self,
        requested: ActionKind,
        effective: ActionKind,
        expected: StateId,
        target: Option<ElementRef>,
        destination: mawr_core::AbsoluteUrl,
        navigation: NavigationRequest,
    ) -> Prepared {
        let method = navigation.method();
        Prepared {
            effective,
            authorization: ActionAuthorizationContext::new(
                OperationKind::Act(requested),
                requested,
                effective,
                expected,
                target,
                Some(destination),
                Some(method),
            ),
            effect: Effect::Network(navigation),
        }
    }

    fn ensure_current(&self, expected: StateId) -> Result<(), OperationFailure> {
        if expected.session() != self.store.session() {
            return Err(OperationFailure::session_mismatch(
                "action_state_session",
                self.store.session(),
                expected.session(),
            ));
        }
        let actual = self.store.current().map(mawr_state::StoredState::id);
        if actual != Some(expected) {
            return Err(OperationFailure::StaleState { expected, actual });
        }
        Ok(())
    }

    fn resolve(
        &self,
        expected: StateId,
        reference: ElementRef,
    ) -> Result<&StoredSemanticUnit, OperationFailure> {
        self.store.resolve_current(expected, reference)
    }

    fn require_affordance(
        &self,
        unit: &StoredSemanticUnit,
        action: ActionKind,
        capability: Capability,
    ) -> Result<(), OperationFailure> {
        if unit.semantic().state().disabled() == &Property::Known(true)
            || !unit.semantic().affordances().contains(action)
        {
            return Err(self.unsupported(capability));
        }
        Ok(())
    }

    fn require_capability(&self, capability: Capability) -> Result<(), OperationFailure> {
        match self.capabilities.status(capability) {
            CapabilityStatus::Supported | CapabilityStatus::Limited(_) => Ok(()),
            CapabilityStatus::Unsupported(reason) => Err(OperationFailure::UnsupportedCapability {
                capability,
                engine: self.capabilities.engine().clone(),
                reason: *reason,
            }),
        }
    }

    fn unsupported(&self, capability: Capability) -> OperationFailure {
        OperationFailure::UnsupportedCapability {
            capability,
            engine: self.capabilities.engine().clone(),
            reason: UnsupportedReason::EngineLimitation,
        }
    }
}

fn known_destination(
    unit: &StoredSemanticUnit,
) -> Result<&mawr_core::AbsoluteUrl, OperationFailure> {
    match unit.semantic().destination() {
        Property::Known(destination) => Ok(destination),
        _ => Err(OperationFailure::invalid_input(
            "action_destination",
            ValidationIssue::InvalidFormat,
        )),
    }
}

fn push_named_value(
    fields: &mut Vec<FormField>,
    interaction: &mawr_semantic_html::StaticInteraction,
) -> Result<(), OperationFailure> {
    let Some(name) = interaction.name() else {
        return Ok(());
    };
    let value = interaction.submission_value().ok_or_else(|| {
        OperationFailure::invalid_input("form_control_value", ValidationIssue::InvalidFormat)
    })?;
    fields.push(FormField::new(name, value).map_err(OperationFailure::InvalidInput)?);
    Ok(())
}

fn append_hidden(
    fields: &mut Vec<FormField>,
    hidden: &mawr_semantic_html::StaticHiddenControl,
) -> Result<(), OperationFailure> {
    if !hidden.supported() {
        return Err(OperationFailure::invalid_input(
            "hidden_form_control",
            ValidationIssue::InvalidFormat,
        ));
    }
    fields.push(
        FormField::new(
            hidden.name().expect("supported hidden name"),
            hidden.value().expect("supported hidden value"),
        )
        .map_err(OperationFailure::InvalidInput)?,
    );
    Ok(())
}

fn invariant(code: &'static str) -> OperationFailure {
    OperationFailure::InvariantViolation {
        code: mawr_core::NonEmptyText::new(code, "invariant_code")
            .expect("static invariant code is valid"),
    }
}
