use std::io;
use std::sync::{Arc, Mutex};

use mawr_actions::{
    ActionAuthorizationContext, ActionAuthorizer, AuthorizationDecision, BatchAuditPhase,
    BatchSkipReason, SideEffectStatus, StaticActionExecutor,
};
use mawr_core::{
    AbsoluteUrl, Action, ActionBatch, ActionKind, ActionRequest, AuthorizationReason,
    BatchFailurePolicy, Capability, CapabilityStatus, ElementRef, FailureClass, ObservationChanges,
    ObservationRequest, OperationFailure, PressCommand, Property, SemanticRole, SessionId,
    TransitionCause,
};
use mawr_native_static::{
    CancellationToken, DestinationPolicy, NativeStaticConfig, NativeStaticEngine, NavigationRequest,
};
use mawr_observation::{FullObservationBuilder, FullObservationConfig, SemanticSnapshot};
use mawr_semantic_html::{HtmlSemanticExtractor, StaticInteractionKind};
use mawr_state::{SemanticStateStore, StateStoreConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapturedRequest {
    method: String,
    target: String,
    body: String,
}

struct FixtureServer {
    port: u16,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    task: JoinHandle<()>,
}

impl FixtureServer {
    async fn spawn() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let captured = Arc::clone(&captured);
                tokio::spawn(async move {
                    let _ = serve(stream, captured).await;
                });
            }
        });
        Self {
            port,
            requests,
            task,
        }
    }

    fn url(&self, path: &str) -> AbsoluteUrl {
        AbsoluteUrl::new(format!("http://127.0.0.1:{}{path}", self.port)).unwrap()
    }

    fn engine(&self) -> NativeStaticEngine {
        NativeStaticEngine::new(
            NativeStaticConfig::default()
                .with_destination_policy(DestinationPolicy::loopback(self.port).unwrap()),
        )
    }

    fn take_requests(&self) -> Vec<CapturedRequest> {
        std::mem::take(&mut *self.requests.lock().unwrap())
    }
}

impl Drop for FixtureServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(
    mut stream: TcpStream,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
) -> io::Result<()> {
    let request = read_request(&mut stream).await?;
    let response = if request.target == "/binary" {
        response("application/octet-stream", b"not-html")
    } else if request.target == "/start" {
        response("text/html; charset=utf-8", START_HTML.as_bytes())
    } else {
        response(
            "text/html; charset=utf-8",
            format!(
                "<title>Result</title><p>{} {} {}</p>",
                request.method, request.target, request.body
            )
            .as_bytes(),
        )
    };
    requests.lock().unwrap().push(request);
    stream.write_all(&response).await?;
    stream.shutdown().await
}

async fn read_request(stream: &mut TcpStream) -> io::Result<CapturedRequest> {
    let mut bytes = Vec::new();
    let header_end;
    loop {
        let mut chunk = [0_u8; 1024];
        let count = stream.read(&mut chunk).await?;
        if count == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            header_end = index + 4;
            break;
        }
    }
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let mut lines = headers.lines();
    let mut request_line = lines.next().unwrap().split_ascii_whitespace();
    let method = request_line.next().unwrap().to_owned();
    let target = request_line.next().unwrap().to_owned();
    let content_length = lines
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    while bytes.len() - header_end < content_length {
        let mut chunk = [0_u8; 1024];
        let count = stream.read(&mut chunk).await?;
        if count == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    Ok(CapturedRequest {
        method,
        target,
        body: String::from_utf8(bytes[header_end..header_end + content_length].to_vec()).unwrap(),
    })
}

fn response(content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}

const START_HTML: &str = r#"<!doctype html><title>Actions</title>
<a id="next" href="/next">Next</a>
<a id="download" href="/binary" download>Download</a>
<a id="binary" href="/binary">Binary</a>
<form id="get-form" action="/search?existing=1" method="get" enctype="multipart/form-data">
  <input type="hidden" name="dup" value="hidden">
  <input id="query" aria-label="Query" name="dup" value="initial" required>
  <input id="agree" type="checkbox" aria-label="Agree" name="flag" value="yes">
  <input id="red" type="radio" aria-label="Red" name="color" value="red">
  <input id="blue" type="radio" aria-label="Blue" name="color" value="blue">
  <select id="choice" aria-label="Choice" name="choice" required>
    <option id="alpha" value="a" selected>Alpha</option>
    <option id="beta" value="b">Beta</option>
  </select>
  <button id="get-submit" name="commit" value="1">Search</button>
  <button id="novalidate-submit" name="skip" value="1" formnovalidate>Skip validation</button>
  <input id="image-submit" type="image" name="image" alt="Image submit">
  <button id="reset" type="reset">Reset</button>
  <button id="plain" type="button">Dynamic</button>
  <button id="disabled" disabled>Disabled</button>
</form>
<form id="post-form" action="/post" method="post">
  <input id="post-value" aria-label="Post value" name="item" value="hello world">
  <button id="post-submit" name="send" value="yes">Post</button>
</form>"#;

async fn executor<A: mawr_actions::ActionAuthorizer>(
    server: &FixtureServer,
    sequence: u64,
    authorizer: A,
) -> StaticActionExecutor<A> {
    let engine = server.engine();
    let session_id = SessionId::new(sequence).unwrap();
    let session = engine.start_session(session_id);
    let input = session
        .navigate(
            NavigationRequest::get(server.url("/start")),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    let extractor = HtmlSemanticExtractor::default();
    let document = extractor.extract(&input).unwrap();
    let mut store = SemanticStateStore::new(
        session_id,
        engine.identity().clone(),
        StateStoreConfig::default(),
    );
    store.update(document, TransitionCause::Navigation).unwrap();
    StaticActionExecutor::new(session, extractor, store, authorizer).unwrap()
}

fn allow(_: &ActionAuthorizationContext) -> AuthorizationDecision {
    AuthorizationDecision::Allow
}

fn deny(_: &ActionAuthorizationContext) -> AuthorizationDecision {
    AuthorizationDecision::Deny(AuthorizationReason::MutationNotGranted)
}

#[derive(Clone)]
struct RecordingAuthorizer {
    contexts: Arc<Mutex<Vec<ActionAuthorizationContext>>>,
    deny_at: Option<usize>,
}

impl RecordingAuthorizer {
    fn allow_all() -> Self {
        Self {
            contexts: Arc::new(Mutex::new(Vec::new())),
            deny_at: None,
        }
    }

    fn deny_at(index: usize) -> Self {
        Self {
            contexts: Arc::new(Mutex::new(Vec::new())),
            deny_at: Some(index),
        }
    }

    fn contexts(&self) -> Vec<ActionAuthorizationContext> {
        self.contexts.lock().unwrap().clone()
    }
}

impl ActionAuthorizer for RecordingAuthorizer {
    fn authorize(&self, context: &ActionAuthorizationContext) -> AuthorizationDecision {
        let mut contexts = self.contexts.lock().unwrap();
        let index = contexts.len();
        contexts.push(context.clone());
        if self.deny_at == Some(index) {
            AuthorizationDecision::Deny(AuthorizationReason::MutationNotGranted)
        } else {
            AuthorizationDecision::Allow
        }
    }
}

fn reference<A: ActionAuthorizer>(executor: &StaticActionExecutor<A>, id: &str) -> ElementRef {
    executor
        .store()
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

fn current_state<A: ActionAuthorizer>(executor: &StaticActionExecutor<A>) -> mawr_core::StateId {
    executor.store().current().unwrap().id()
}

fn request<A: ActionAuthorizer>(
    executor: &StaticActionExecutor<A>,
    action: Action,
) -> ActionRequest {
    ActionRequest::new(current_state(executor), action).unwrap()
}

#[tokio::test]
async fn local_controls_mutate_deterministically_and_preserve_references() {
    let server = FixtureServer::spawn().await;
    let mut executor = executor(&server, 1, allow).await;
    server.take_requests();
    for capability in [
        Capability::TextInput,
        Capability::Checkbox,
        Capability::Radio,
        Capability::Select,
        Capability::Button,
    ] {
        assert!(matches!(
            executor.capabilities().status(capability),
            CapabilityStatus::Limited(_)
        ));
    }

    let query = reference(&executor, "query");
    let fill = request(&executor, Action::fill(query, "private-value").unwrap());
    let outcome = executor
        .execute(fill, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(outcome.requested(), ActionKind::Fill);
    assert_eq!(outcome.effective(), ActionKind::Fill);
    assert!(outcome.network().is_none());
    assert_eq!(reference(&executor, "query"), query);
    assert!(!format!("{outcome:?}").contains("private-value"));

    let agree = reference(&executor, "agree");
    executor
        .execute(
            request(&executor, Action::check(agree)),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    let agree_unit = executor.store().current().unwrap().unit(agree).unwrap();
    assert_eq!(
        agree_unit.semantic().state().checked(),
        &Property::Known(true)
    );
    assert!(
        agree_unit
            .semantic()
            .affordances()
            .contains(ActionKind::Uncheck)
    );
    executor
        .execute(
            request(&executor, Action::uncheck(agree)),
            &CancellationToken::new(),
        )
        .await
        .unwrap();

    let red = reference(&executor, "red");
    let blue = reference(&executor, "blue");
    executor
        .execute(
            request(&executor, Action::check(red)),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    executor
        .execute(
            request(&executor, Action::check(blue)),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(
        executor
            .store()
            .current()
            .unwrap()
            .unit(red)
            .unwrap()
            .semantic()
            .state()
            .checked(),
        &Property::Known(false)
    );
    assert_eq!(
        executor
            .store()
            .current()
            .unwrap()
            .unit(blue)
            .unwrap()
            .semantic()
            .state()
            .checked(),
        &Property::Known(true)
    );

    let choice = reference(&executor, "choice");
    let beta = reference(&executor, "beta");
    executor
        .execute(
            request(&executor, Action::select(choice, beta).unwrap()),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(
        executor
            .store()
            .current()
            .unwrap()
            .unit(beta)
            .unwrap()
            .semantic()
            .state()
            .selected(),
        &Property::Known(true)
    );
    assert!(server.take_requests().is_empty());
}

#[tokio::test]
async fn get_and_post_submit_successful_controls_in_document_order() {
    let server = FixtureServer::spawn().await;
    let mut get_executor = executor(&server, 2, allow).await;
    server.take_requests();
    let get_submit = reference(&get_executor, "get-submit");
    let outcome = get_executor
        .execute(
            request(&get_executor, Action::submit(get_submit)),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(outcome.network().unwrap().status(), 200);
    assert_eq!(
        outcome.network().unwrap().method(),
        mawr_native_static::RequestMethod::Get
    );
    assert_eq!(
        server.take_requests(),
        vec![CapturedRequest {
            method: "GET".to_owned(),
            target: "/search?existing=1&dup=hidden&dup=initial&choice=a&commit=1".to_owned(),
            body: String::new(),
        }]
    );

    let mut post_executor = executor(&server, 3, allow).await;
    server.take_requests();
    let post_submit = reference(&post_executor, "post-submit");
    post_executor
        .execute(
            request(&post_executor, Action::submit(post_submit)),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(
        server.take_requests(),
        vec![CapturedRequest {
            method: "POST".to_owned(),
            target: "/post".to_owned(),
            body: "item=hello+world&send=yes".to_owned(),
        }]
    );
}

#[tokio::test]
async fn get_ignores_enctype_and_formnovalidate_bypasses_constraint_checks() {
    let server = FixtureServer::spawn().await;
    let mut executor = executor(&server, 30, allow).await;
    server.take_requests();
    let query = reference(&executor, "query");
    executor
        .execute(
            request(&executor, Action::fill(query, "").unwrap()),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    let submit = reference(&executor, "novalidate-submit");
    executor
        .execute(
            request(&executor, Action::submit(submit)),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(
        server.take_requests(),
        vec![CapturedRequest {
            method: "GET".to_owned(),
            target: "/search?existing=1&dup=hidden&dup=&choice=a&skip=1".to_owned(),
            body: String::new(),
        }]
    );
}

#[tokio::test]
async fn press_maps_only_supported_static_semantics() {
    let server = FixtureServer::spawn().await;
    let mut executor = executor(&server, 4, allow).await;
    server.take_requests();
    let agree = reference(&executor, "agree");
    let outcome = executor
        .execute(
            request(&executor, Action::press(Some(agree), PressCommand::Space)),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(outcome.requested(), ActionKind::Press);
    assert_eq!(outcome.effective(), ActionKind::Check);

    let plain = reference(&executor, "plain");
    let failure = executor
        .execute(
            request(&executor, Action::press(Some(plain), PressCommand::Enter)),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        failure.failure(),
        OperationFailure::UnsupportedCapability {
            capability: Capability::JavaScript,
            ..
        }
    ));
    assert_eq!(failure.side_effect(), SideEffectStatus::NotStarted);

    let image = reference(&executor, "image-submit");
    let image_failure = executor
        .execute(
            request(&executor, Action::press(Some(image), PressCommand::Enter)),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        image_failure.failure(),
        OperationFailure::UnsupportedCapability {
            capability: Capability::Geometry,
            ..
        }
    ));

    let next = reference(&executor, "next");
    let followed = executor
        .execute(
            request(&executor, Action::press(Some(next), PressCommand::Enter)),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(followed.effective(), ActionKind::Follow);
    assert_eq!(
        followed.network().unwrap().final_url().as_str(),
        server.url("/next").as_str()
    );
}

#[tokio::test]
async fn every_preflight_rejection_is_side_effect_free() {
    let server = FixtureServer::spawn().await;
    let mut denied = executor(&server, 5, deny).await;
    server.take_requests();
    let query = reference(&denied, "query");
    let original = current_state(&denied);
    let failure = denied
        .execute(
            request(&denied, Action::fill(query, "denied-secret").unwrap()),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert_eq!(failure.side_effect(), SideEffectStatus::NotStarted);
    assert_eq!(failure.failure().class(), FailureClass::AuthorizationDenied);
    assert_eq!(current_state(&denied), original);
    assert!(!format!("{failure:?}").contains("denied-secret"));
    let denied_link = reference(&denied, "next");
    let denied_navigation = denied
        .execute(
            request(&denied, Action::follow(denied_link)),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        denied_navigation.side_effect(),
        SideEffectStatus::NotStarted
    );

    let mut executor = executor(&server, 6, allow).await;
    server.take_requests();
    let disabled = reference(&executor, "disabled");
    let disabled_failure = executor
        .execute(
            request(&executor, Action::submit(disabled)),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert_eq!(disabled_failure.side_effect(), SideEffectStatus::NotStarted);

    let download = reference(&executor, "download");
    let download_failure = executor
        .execute(
            request(&executor, Action::follow(download)),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        download_failure.failure(),
        OperationFailure::AuthorizationDenied {
            operation: mawr_core::OperationKind::Download,
            reason: AuthorizationReason::ConfirmationRequired,
        }
    ));

    let missing = ElementRef::new(SessionId::new(6).unwrap(), 999_999).unwrap();
    let missing_failure = executor
        .execute(
            request(&executor, Action::follow(missing)),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        missing_failure.failure().class(),
        FailureClass::MissingReference
    );

    let query = reference(&executor, "query");
    executor
        .execute(
            request(&executor, Action::fill(query, "").unwrap()),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    let get_form = reference(&executor, "get-form");
    let invalid = executor
        .execute(
            request(&executor, Action::submit(get_form)),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert_eq!(invalid.failure().class(), FailureClass::InvalidInput);
    assert_eq!(invalid.side_effect(), SideEffectStatus::NotStarted);

    let old_state = current_state(&executor);
    let agree = reference(&executor, "agree");
    executor
        .execute(
            request(&executor, Action::check(agree)),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    let stale = ActionRequest::new(old_state, Action::uncheck(agree)).unwrap();
    let stale_failure = executor
        .execute(stale, &CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(stale_failure.failure().class(), FailureClass::StaleState);
    assert!(server.take_requests().is_empty());
}

#[tokio::test]
async fn completed_network_evidence_survives_non_html_parse_failure() {
    let server = FixtureServer::spawn().await;
    let mut executor = executor(&server, 7, allow).await;
    server.take_requests();
    let binary = reference(&executor, "binary");
    let previous = current_state(&executor);
    let failure = executor
        .execute(
            request(&executor, Action::follow(binary)),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert_eq!(failure.side_effect(), SideEffectStatus::NetworkCompleted);
    assert_eq!(failure.failure().class(), FailureClass::Parsing);
    assert_eq!(failure.network().unwrap().status(), 200);
    assert_eq!(failure.network().unwrap().decoded_body_bytes(), 8);
    assert_eq!(current_state(&executor), previous);
}

#[tokio::test]
async fn constructor_and_requests_enforce_session_isolation() {
    let server = FixtureServer::spawn().await;
    let engine = server.engine();
    let first = SessionId::new(80).unwrap();
    let second = SessionId::new(81).unwrap();
    let store = SemanticStateStore::new(
        second,
        engine.identity().clone(),
        StateStoreConfig::default(),
    );
    let result = StaticActionExecutor::new(
        engine.start_session(first),
        HtmlSemanticExtractor::default(),
        store,
        allow,
    );
    assert!(matches!(result, Err(OperationFailure::InvalidInput(_))));

    let mut executor = executor(&server, 82, allow).await;
    let foreign_state = mawr_core::StateId::new(SessionId::new(83).unwrap(), 1).unwrap();
    let failure = executor
        .execute(
            ActionRequest::new(foreign_state, Action::navigate(server.url("/next"))).unwrap(),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert_eq!(failure.failure().class(), FailureClass::InvalidInput);

    let navigated = executor
        .execute(
            request(&executor, Action::navigate(server.url("/next"))),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(navigated.requested(), ActionKind::Navigate);
    assert_eq!(
        navigated.network().unwrap().final_url().as_str(),
        server.url("/next").as_str()
    );
}

#[test]
fn semantic_metadata_is_secret_safe_and_native_only() {
    let session = SessionId::new(90).unwrap();
    let url = AbsoluteUrl::new("https://example.test/").unwrap();
    let document = HtmlSemanticExtractor::default()
        .extract_source(mawr_semantic_html::HtmlDocumentSource::new(
            session,
            &url,
            br#"<form><input type="hidden" name="token" value="hidden-secret"><input id="password" type="password" name="password" value="field-secret"><div id="fake" role="textbox">fake</div></form>"#,
        ))
        .unwrap();
    let password = document
        .units()
        .iter()
        .find(|unit| matches!(unit.author_id(), Property::Known(id) if id.as_str() == "password"))
        .unwrap();
    let fake = document
        .units()
        .iter()
        .find(|unit| matches!(unit.author_id(), Property::Known(id) if id.as_str() == "fake"))
        .unwrap();
    assert_eq!(
        password.interaction().kind(),
        StaticInteractionKind::TextControl
    );
    assert_eq!(fake.role(), SemanticRole::Textbox);
    assert_eq!(fake.interaction().kind(), StaticInteractionKind::None);
    assert!(!fake.affordances().contains(ActionKind::Fill));
    let debug = format!("{document:?} {password:?} {:?}", document.hidden_controls());
    assert!(!debug.contains("hidden-secret"));
    assert!(!debug.contains("field-secret"));
}

#[tokio::test]
async fn valid_dependent_batch_matches_sequential_execution() {
    let server = FixtureServer::spawn().await;
    let authorizer = RecordingAuthorizer::allow_all();
    let mut batched = executor(&server, 100, authorizer.clone()).await;
    server.take_requests();
    let initial = current_state(&batched);
    let query = reference(&batched, "query");
    let agree = reference(&batched, "agree");
    let submit = reference(&batched, "get-submit");
    let batch = ActionBatch::new(
        initial,
        vec![
            Action::fill(query, "batched value").unwrap(),
            Action::check(agree),
            Action::submit(submit),
        ],
        BatchFailurePolicy::StopOnFailure,
    )
    .unwrap();
    let outcome = batched
        .execute_batch(batch, &CancellationToken::new())
        .await
        .unwrap();
    let batched_requests = server.take_requests();

    assert_eq!(outcome.initial_state(), initial);
    assert_ne!(outcome.final_state(), initial);
    assert_eq!(outcome.items().len(), 3);
    assert!(
        outcome
            .items()
            .iter()
            .all(|item| item.succeeded().is_some())
    );
    assert_eq!(outcome.diagnostics().action_count(), 3);
    assert_eq!(outcome.diagnostics().executed_count(), 3);
    assert_eq!(outcome.diagnostics().failure_count(), 0);
    assert_eq!(outcome.diagnostics().decision_boundaries_avoided(), 2);
    assert_eq!(authorizer.contexts().len(), 3);
    assert_eq!(
        outcome
            .audit_events()
            .iter()
            .filter(|event| event.phase() == BatchAuditPhase::Authorized)
            .count(),
        3
    );
    assert_eq!(
        batched_requests,
        vec![CapturedRequest {
            method: "GET".to_owned(),
            target: "/search?existing=1&dup=hidden&dup=batched+value&flag=yes&choice=a&commit=1"
                .to_owned(),
            body: String::new(),
        }]
    );

    let mut sequential = executor(&server, 101, allow).await;
    server.take_requests();
    let query = reference(&sequential, "query");
    sequential
        .execute(
            request(&sequential, Action::fill(query, "batched value").unwrap()),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    let agree = reference(&sequential, "agree");
    sequential
        .execute(
            request(&sequential, Action::check(agree)),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    let submit = reference(&sequential, "get-submit");
    sequential
        .execute(
            request(&sequential, Action::submit(submit)),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(server.take_requests(), batched_requests);
}

#[tokio::test]
async fn invalid_middle_and_stale_batches_execute_nothing() {
    let server = FixtureServer::spawn().await;
    let authorizer = RecordingAuthorizer::allow_all();
    let mut executor = executor(&server, 102, authorizer.clone()).await;
    server.take_requests();
    let initial = current_state(&executor);
    let query = reference(&executor, "query");
    let agree = reference(&executor, "agree");
    let missing = ElementRef::new(SessionId::new(102).unwrap(), 999_999).unwrap();
    let invalid = ActionBatch::new(
        initial,
        vec![
            Action::fill(query, "must-not-commit").unwrap(),
            Action::follow(missing),
            Action::check(agree),
        ],
        BatchFailurePolicy::StopOnFailure,
    )
    .unwrap();
    let failure = executor
        .execute_batch(invalid, &CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(failure.index(), 1);
    assert_eq!(
        failure.failure().failure().class(),
        FailureClass::MissingReference
    );
    let rejected = failure.audit_events().last().unwrap();
    assert_eq!(rejected.phase(), BatchAuditPhase::PreflightRejected);
    assert_eq!(rejected.target(), Some(missing));
    assert_eq!(rejected.side_effect(), Some(SideEffectStatus::NotStarted));
    assert_eq!(current_state(&executor), initial);
    assert!(server.take_requests().is_empty());
    assert_eq!(authorizer.contexts().len(), 1);
    assert!(!format!("{failure:?}").contains("must-not-commit"));

    executor
        .execute(
            ActionRequest::new(initial, Action::check(agree)).unwrap(),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    let after_check = current_state(&executor);
    let stale = ActionBatch::new(
        initial,
        vec![Action::uncheck(agree)],
        BatchFailurePolicy::StopOnFailure,
    )
    .unwrap();
    let stale_failure = executor
        .execute_batch(stale, &CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(stale_failure.index(), 0);
    assert_eq!(
        stale_failure.failure().failure().class(),
        FailureClass::StaleState
    );
    assert_eq!(current_state(&executor), after_check);
    assert!(server.take_requests().is_empty());
}

#[tokio::test]
async fn reference_after_navigation_boundary_is_rejected_before_network() {
    let server = FixtureServer::spawn().await;
    let mut executor = executor(&server, 103, allow).await;
    server.take_requests();
    let initial = current_state(&executor);
    let next = reference(&executor, "next");
    let query = reference(&executor, "query");
    let batch = ActionBatch::new(
        initial,
        vec![
            Action::follow(next),
            Action::fill(query, "unreachable").unwrap(),
        ],
        BatchFailurePolicy::StopOnFailure,
    )
    .unwrap();
    let failure = executor
        .execute_batch(batch, &CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(failure.index(), 1);
    assert_eq!(
        failure.failure().failure().class(),
        FailureClass::InvalidInput
    );
    assert_eq!(current_state(&executor), initial);
    assert!(server.take_requests().is_empty());
}

#[tokio::test]
async fn runtime_partial_result_honors_both_failure_policies_and_retry_is_stale() {
    let server = FixtureServer::spawn().await;
    let mut stopped = executor(&server, 104, allow).await;
    server.take_requests();
    let initial = current_state(&stopped);
    let query = reference(&stopped, "query");
    let binary = reference(&stopped, "binary");
    let actions = vec![
        Action::fill(query, "committed-prefix").unwrap(),
        Action::follow(binary),
        Action::navigate(server.url("/next")),
    ];
    let stop_batch =
        ActionBatch::new(initial, actions.clone(), BatchFailurePolicy::StopOnFailure).unwrap();
    let outcome = stopped
        .execute_batch(stop_batch.clone(), &CancellationToken::new())
        .await
        .unwrap();
    assert!(outcome.items()[0].succeeded().is_some());
    let failure = outcome.items()[1].failure().unwrap();
    assert_eq!(failure.failure().class(), FailureClass::Parsing);
    assert_eq!(failure.side_effect(), SideEffectStatus::NetworkCompleted);
    assert_eq!(
        outcome.items()[2].skip_reason(),
        Some(BatchSkipReason::PriorFailure)
    );
    assert_eq!(outcome.diagnostics().executed_count(), 2);
    assert_eq!(outcome.diagnostics().failure_count(), 1);
    assert_eq!(server.take_requests()[0].target, "/binary");
    let committed = current_state(&stopped);
    assert_ne!(committed, initial);

    let retry = stopped
        .execute_batch(stop_batch, &CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(retry.failure().failure().class(), FailureClass::StaleState);
    assert_eq!(current_state(&stopped), committed);

    let mut continued = executor(&server, 105, allow).await;
    server.take_requests();
    let initial = current_state(&continued);
    let query = reference(&continued, "query");
    let binary = reference(&continued, "binary");
    let continue_batch = ActionBatch::new(
        initial,
        vec![
            Action::fill(query, "committed-prefix").unwrap(),
            Action::follow(binary),
            Action::navigate(server.url("/next")),
        ],
        BatchFailurePolicy::ContinueIndependent,
    )
    .unwrap();
    let continued_outcome = continued
        .execute_batch(continue_batch, &CancellationToken::new())
        .await
        .unwrap();
    assert!(continued_outcome.items()[0].succeeded().is_some());
    assert!(continued_outcome.items()[1].failure().is_some());
    assert!(continued_outcome.items()[2].succeeded().is_some());
    assert_eq!(continued_outcome.diagnostics().executed_count(), 3);
    assert_eq!(continued_outcome.diagnostics().failure_count(), 1);
    assert_eq!(
        continued.store().current().unwrap().page().url().as_str(),
        server.url("/next").as_str()
    );
    assert_eq!(
        server
            .take_requests()
            .into_iter()
            .map(|request| request.target)
            .collect::<Vec<_>>(),
        vec!["/binary".to_owned(), "/next".to_owned()]
    );
}

#[tokio::test]
async fn authorization_denial_is_atomic_redacted_and_concurrent_batches_go_stale() {
    let server = FixtureServer::spawn().await;
    let authorizer = RecordingAuthorizer::deny_at(1);
    let mut denied = executor(&server, 106, authorizer.clone()).await;
    server.take_requests();
    let initial = current_state(&denied);
    let query = reference(&denied, "query");
    let agree = reference(&denied, "agree");
    let batch = ActionBatch::new(
        initial,
        vec![
            Action::fill(query, "authorization-secret").unwrap(),
            Action::check(agree),
        ],
        BatchFailurePolicy::StopOnFailure,
    )
    .unwrap();
    let failure = denied
        .execute_batch(batch, &CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(failure.index(), 1);
    assert_eq!(
        failure.failure().failure().class(),
        FailureClass::AuthorizationDenied
    );
    assert_eq!(authorizer.contexts().len(), 2);
    assert_eq!(current_state(&denied), initial);
    assert!(server.take_requests().is_empty());
    assert!(!format!("{failure:?}").contains("authorization-secret"));

    let mut executor = executor(&server, 107, allow).await;
    server.take_requests();
    let initial = current_state(&executor);
    let agree = reference(&executor, "agree");
    let first = ActionBatch::new(
        initial,
        vec![Action::check(agree)],
        BatchFailurePolicy::StopOnFailure,
    )
    .unwrap();
    let competing = ActionBatch::new(
        initial,
        vec![Action::check(agree)],
        BatchFailurePolicy::StopOnFailure,
    )
    .unwrap();
    let winner = executor
        .execute_batch(first, &CancellationToken::new())
        .await
        .unwrap();
    let winner_state = winner.final_state();
    let loser = executor
        .execute_batch(competing, &CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(loser.failure().failure().class(), FailureClass::StaleState);
    assert_eq!(current_state(&executor), winner_state);
    assert!(server.take_requests().is_empty());
}

#[tokio::test]
async fn local_action_transition_produces_a_reconstructable_validation_diff() {
    let server = FixtureServer::spawn().await;
    let mut executor = executor(&server, 108, allow).await;
    server.take_requests();
    let builder = FullObservationBuilder::new(
        executor.capabilities().clone(),
        FullObservationConfig::default(),
    );
    let base_state = current_state(&executor);
    let base = builder
        .build(
            executor.store(),
            &ObservationRequest::new(SessionId::new(108).unwrap()),
        )
        .unwrap()
        .into_observation();
    let query = reference(&executor, "query");
    executor
        .execute(
            request(&executor, Action::fill(query, "").unwrap()),
            &CancellationToken::new(),
        )
        .await
        .unwrap();

    let incremental = builder
        .build(
            executor.store(),
            &ObservationRequest::new(SessionId::new(108).unwrap())
                .since_state(base_state)
                .unwrap(),
        )
        .unwrap();
    let ObservationChanges::Computed(changes) = incremental.observation().changes() else {
        panic!("local action must produce computed semantic changes");
    };
    assert_eq!(changes.updated(), &[query]);
    let changed_query = incremental.observation().units().first().unwrap();
    assert_eq!(changed_query.reference(), query);
    assert_eq!(changed_query.state().invalid(), &Property::Known(true));
    assert!(server.take_requests().is_empty());

    let reconstructed = SemanticSnapshot::from_full(&base)
        .unwrap()
        .apply(incremental.observation())
        .unwrap();
    let target = builder
        .build(
            executor.store(),
            &ObservationRequest::new(SessionId::new(108).unwrap()),
        )
        .unwrap();
    assert_eq!(
        reconstructed,
        SemanticSnapshot::from_full(target.observation()).unwrap()
    );
}
