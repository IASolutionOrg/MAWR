use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use flate2::Compression;
use flate2::write::GzEncoder;
use mawr_core::{
    AbsoluteUrl, AuthorizationReason, FailureClass, Measurement, MeasurementKind,
    MeasurementSource, NavigationFailureKind, OperationFailure, ResourceKind, SessionId,
    UnavailableReason,
};
use mawr_native_static::{
    CancellationToken, DestinationPolicy, DownloadPolicy, DownloadRequest, FormField, FormMethod,
    FormSubmission, NativeStaticConfig, NativeStaticEngine, NavigationRequest, SafeFilename,
    TransportLimits,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct FixtureServer {
    port: u16,
    task: JoinHandle<()>,
}

impl FixtureServer {
    async fn spawn() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let _ = serve_connection(stream).await;
                });
            }
        });
        Self { port, task }
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

    fn engine_with_limits(&self, limits: TransportLimits) -> NativeStaticEngine {
        NativeStaticEngine::new(
            NativeStaticConfig::default()
                .with_destination_policy(DestinationPolicy::loopback(self.port).unwrap())
                .with_limits(limits),
        )
    }
}

impl Drop for FixtureServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Debug)]
struct FixtureRequest {
    method: String,
    target: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

async fn serve_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let request = read_request(&mut stream).await?;
    if request.target == "/slow-download" {
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 20\r\n\r\npartial")
            .await?;
        stream.flush().await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
        stream.write_all(b"-download-end").await?;
        return stream.shutdown().await;
    }
    let response = fixture_response(&request).await;
    stream.write_all(&response).await?;
    stream.shutdown().await
}

async fn read_request(stream: &mut TcpStream) -> std::io::Result<FixtureRequest> {
    let mut bytes = Vec::new();
    let header_end = loop {
        let mut chunk = [0_u8; 1024];
        let count = stream.read(&mut chunk).await?;
        if count == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() > 1024 * 1024 {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        }
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
    };
    let header_text = std::str::from_utf8(&bytes[..header_end - 4])
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or("").to_owned();
    let target = request_parts.next().unwrap_or("").to_owned();
    let mut headers = BTreeMap::new();
    for line in lines {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
        headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
    }
    let content_length = headers
        .get("content-length")
        .map_or(Ok(0), |value| value.parse::<usize>())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
    while bytes.len() - header_end < content_length {
        let mut chunk = [0_u8; 1024];
        let count = stream.read(&mut chunk).await?;
        if count == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    Ok(FixtureRequest {
        method,
        target,
        headers,
        body: bytes[header_end..header_end + content_length].to_vec(),
    })
}

async fn fixture_response(request: &FixtureRequest) -> Vec<u8> {
    match request.target.as_str() {
        "/get" => response(
            200,
            &[],
            format!("{} {}", request.method, request.target).as_bytes(),
        ),
        "/head" => response(200, &[("Content-Length", "4")], &[]),
        "/metadata" => response(
            200,
            &[
                ("Content-Type", "text/html; charset=utf-8"),
                ("Content-Language", "it"),
            ],
            b"<title>MAWR</title>",
        ),
        "/redirect" => response(302, &[("Location", "/final")], b"redirect"),
        "/final" => response(200, &[], b"final"),
        "/post-302" => response(302, &[("Location", "/method")], b"redirect"),
        "/post-307" => response(307, &[("Location", "/echo")], b"redirect"),
        "/method" => response(200, &[], request.method.as_bytes()),
        "/echo" => response(
            200,
            &[],
            format!(
                "{}\n{}",
                request.method,
                String::from_utf8_lossy(&request.body)
            )
            .as_bytes(),
        ),
        "/loop-a" => response(302, &[("Location", "/loop-b")], b""),
        "/loop-b" => response(302, &[("Location", "/loop-a")], b""),
        "/chain/3" => response(302, &[("Location", "/chain/2")], b""),
        "/chain/2" => response(302, &[("Location", "/chain/1")], b""),
        "/chain/1" => response(302, &[("Location", "/final")], b""),
        "/missing-location" => response(302, &[], b""),
        "/extra-header" => response(200, &[("X-Fixture", "present")], b"headers"),
        "/cookie/set" => response(
            200,
            &[("Set-Cookie", "sid=alpha; Path=/; HttpOnly")],
            b"set",
        ),
        "/cookie/check" => response(
            200,
            &[],
            request
                .headers
                .get("cookie")
                .map_or(b"".as_slice(), String::as_bytes),
        ),
        "/cookie/oversized" => response(
            200,
            &[("Set-Cookie", "oversized=abcdefghijklmnopqrstuvwxyz; Path=/")],
            b"cookie",
        ),
        "/gzip" => {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&vec![b'x'; 4096]).unwrap();
            let compressed = encoder.finish().unwrap();
            response_owned(200, vec![("Content-Encoding", "gzip")], compressed)
        }
        "/large" => response(200, &[], &vec![b'x'; 4096]),
        "/slow" => {
            tokio::time::sleep(Duration::from_millis(300)).await;
            response(200, &[], b"slow")
        }
        "/download" => response(200, &[], b"download-content"),
        "/private-redirect" => response(
            302,
            &[("Location", "http://169.254.169.254/latest/meta-data")],
            b"",
        ),
        target if target.starts_with("/form-get?") => response(200, &[], target.as_bytes()),
        _ => response(404, &[], b"not-found"),
    }
}

fn response(status: u16, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    response_owned(status, headers.to_vec(), body.to_vec())
}

fn response_owned(status: u16, headers: Vec<(&str, &str)>, body: Vec<u8>) -> Vec<u8> {
    let reason = match status {
        200 => "OK",
        302 => "Found",
        307 => "Temporary Redirect",
        404 => "Not Found",
        _ => "Fixture",
    };
    let mut response = format!("HTTP/1.1 {status} {reason}\r\nConnection: close\r\n");
    if !headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("content-length"))
    {
        response.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    for (name, value) in headers {
        response.push_str(name);
        response.push_str(": ");
        response.push_str(value);
        response.push_str("\r\n");
    }
    response.push_str("\r\n");
    let mut bytes = response.into_bytes();
    bytes.extend_from_slice(&body);
    bytes
}

#[tokio::test]
async fn supports_get_head_and_response_metadata() {
    let server = FixtureServer::spawn().await;
    let session = server.engine().start_session(SessionId::new(1).unwrap());
    let cancellation = CancellationToken::new();

    let get = session
        .navigate(NavigationRequest::get(server.url("/get")), &cancellation)
        .await
        .unwrap();
    assert_eq!(get.body(), b"GET /get");
    assert_eq!(get.diagnostics().request_count(), 1);
    assert_eq!(get.diagnostics().decoded_body_bytes(), 8);
    let measurements = get.diagnostics().measurements();
    assert!(matches!(
        measurements.get(MeasurementKind::LatencyMicros),
        Measurement::Exact {
            source: MeasurementSource::RuntimeCounter,
            ..
        }
    ));
    assert_eq!(
        measurements.get(MeasurementKind::NetworkBytes),
        &Measurement::Unavailable(UnavailableReason::SourceMissing)
    );
    assert_eq!(
        measurements.get(MeasurementKind::CpuMicros),
        &Measurement::Unavailable(UnavailableReason::NotMeasured)
    );
    assert_eq!(
        measurements.get(MeasurementKind::Retries),
        &Measurement::Exact {
            value: 0,
            source: MeasurementSource::RuntimeCounter,
        }
    );

    let head = session
        .navigate(NavigationRequest::head(server.url("/head")), &cancellation)
        .await
        .unwrap();
    assert!(head.body().is_empty());

    let metadata = session
        .navigate(
            NavigationRequest::get(server.url("/metadata")),
            &cancellation,
        )
        .await
        .unwrap();
    assert_eq!(
        metadata.metadata().content_type(),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(metadata.metadata().content_language(), Some("it"));
    assert!(!format!("{metadata:?}").contains("<title>"));
}

#[tokio::test]
async fn form_get_and_post_are_url_encoded_without_debug_leaks() {
    let server = FixtureServer::spawn().await;
    let session = server.engine().start_session(SessionId::new(2).unwrap());
    let fields = FormSubmission::new(vec![
        FormField::new("query", "hello world").unwrap(),
        FormField::new("token", "sensitive-value").unwrap(),
    ])
    .unwrap();
    let cancellation = CancellationToken::new();

    let get_request = NavigationRequest::submit_form(
        server.url("/form-get?existing=1"),
        FormMethod::Get,
        fields.clone(),
    );
    assert!(!format!("{get_request:?}").contains("sensitive-value"));
    let get = session.navigate(get_request, &cancellation).await.unwrap();
    assert_eq!(
        String::from_utf8_lossy(get.body()),
        "/form-get?existing=1&query=hello+world&token=sensitive-value"
    );

    let post = session
        .navigate(
            NavigationRequest::submit_form(server.url("/echo"), FormMethod::Post, fields),
            &cancellation,
        )
        .await
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(post.body()),
        "POST\nquery=hello+world&token=sensitive-value"
    );
}

#[tokio::test]
async fn redirects_are_manual_bounded_and_method_aware() {
    let server = FixtureServer::spawn().await;
    let cancellation = CancellationToken::new();
    let session = server.engine().start_session(SessionId::new(3).unwrap());

    let simple = session
        .navigate(
            NavigationRequest::get(server.url("/redirect")),
            &cancellation,
        )
        .await
        .unwrap();
    assert_eq!(simple.body(), b"final");
    assert_eq!(simple.redirects().len(), 1);
    assert_eq!(simple.diagnostics().request_count(), 2);

    let form = FormSubmission::new(vec![FormField::new("a", "b").unwrap()]).unwrap();
    let rewritten = session
        .navigate(
            NavigationRequest::submit_form(server.url("/post-302"), FormMethod::Post, form.clone()),
            &cancellation,
        )
        .await
        .unwrap();
    assert_eq!(rewritten.body(), b"GET");

    let preserved = session
        .navigate(
            NavigationRequest::submit_form(server.url("/post-307"), FormMethod::Post, form),
            &cancellation,
        )
        .await
        .unwrap();
    assert_eq!(preserved.body(), b"POST\na=b");

    let loop_error = session
        .navigate(NavigationRequest::get(server.url("/loop-a")), &cancellation)
        .await
        .unwrap_err();
    assert_eq!(
        loop_error,
        OperationFailure::NavigationFailure(NavigationFailureKind::RedirectLoop)
    );

    let limited = server
        .engine_with_limits(TransportLimits::default().with_max_redirects(2).unwrap())
        .start_session(SessionId::new(4).unwrap());
    let limit_error = limited
        .navigate(
            NavigationRequest::get(server.url("/chain/3")),
            &cancellation,
        )
        .await
        .unwrap_err();
    assert_eq!(
        limit_error,
        OperationFailure::NavigationFailure(NavigationFailureKind::TooManyRedirects)
    );

    let missing = session
        .navigate(
            NavigationRequest::get(server.url("/missing-location")),
            &cancellation,
        )
        .await
        .unwrap_err();
    assert_eq!(
        missing,
        OperationFailure::NavigationFailure(NavigationFailureKind::MissingRedirectLocation)
    );
}

#[tokio::test]
async fn cookie_state_is_isolated_per_session() {
    let server = FixtureServer::spawn().await;
    let engine = server.engine();
    let first = engine.start_session(SessionId::new(10).unwrap());
    let second = engine.start_session(SessionId::new(11).unwrap());
    let cancellation = CancellationToken::new();

    first
        .navigate(
            NavigationRequest::get(server.url("/cookie/set")),
            &cancellation,
        )
        .await
        .unwrap();
    let first_cookie = first
        .navigate(
            NavigationRequest::get(server.url("/cookie/check")),
            &cancellation,
        )
        .await
        .unwrap();
    let second_cookie = second
        .navigate(
            NavigationRequest::get(server.url("/cookie/check")),
            &cancellation,
        )
        .await
        .unwrap();
    assert_eq!(first_cookie.body(), b"sid=alpha");
    assert!(second_cookie.body().is_empty());
}

#[tokio::test]
async fn cookie_state_has_a_session_byte_budget() {
    let server = FixtureServer::spawn().await;
    let limits = TransportLimits::default().with_max_cookie_bytes(8).unwrap();
    let session = server
        .engine_with_limits(limits)
        .start_session(SessionId::new(12).unwrap());
    let error = session
        .navigate(
            NavigationRequest::get(server.url("/cookie/oversized")),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        OperationFailure::ResourceLimit {
            resource: ResourceKind::SessionCookies,
            ..
        }
    ));
    let subsequent = session
        .navigate(
            NavigationRequest::get(server.url("/get")),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        subsequent,
        OperationFailure::ResourceLimit {
            resource: ResourceKind::SessionCookies,
            ..
        }
    ));
}

#[tokio::test]
async fn decoded_and_declared_body_limits_fail_closed() {
    let server = FixtureServer::spawn().await;
    let limits = TransportLimits::default()
        .with_max_response_bytes(64)
        .unwrap();
    let session = server
        .engine_with_limits(limits)
        .start_session(SessionId::new(20).unwrap());
    let cancellation = CancellationToken::new();

    for path in ["/large", "/gzip"] {
        let error = session
            .navigate(NavigationRequest::get(server.url(path)), &cancellation)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            OperationFailure::ResourceLimit {
                resource: ResourceKind::ResponseBytes,
                ..
            }
        ));
    }
}

#[tokio::test]
async fn header_and_form_limits_are_enforced_before_consumption() {
    let server = FixtureServer::spawn().await;
    let header_limits = TransportLimits::default().with_max_header_count(2).unwrap();
    let header_session = server
        .engine_with_limits(header_limits)
        .start_session(SessionId::new(21).unwrap());
    let header_error = header_session
        .navigate(
            NavigationRequest::get(server.url("/extra-header")),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        header_error,
        OperationFailure::ResourceLimit {
            resource: ResourceKind::ResponseHeaders,
            ..
        }
    ));

    let form_limits = TransportLimits::default().with_max_form_bytes(4).unwrap();
    let form_session = server
        .engine_with_limits(form_limits)
        .start_session(SessionId::new(22).unwrap());
    let submission =
        FormSubmission::new(vec![FormField::new("name", "long-value").unwrap()]).unwrap();
    let form_error = form_session
        .navigate(
            NavigationRequest::submit_form(server.url("/echo"), FormMethod::Post, submission),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        form_error,
        OperationFailure::ResourceLimit {
            resource: ResourceKind::FormBytes,
            ..
        }
    ));
}

#[tokio::test]
async fn timeout_and_cancellation_are_distinct() {
    let server = FixtureServer::spawn().await;
    let limits = TransportLimits::default()
        .with_total_timeout(Duration::from_millis(50))
        .unwrap();
    let session = server
        .engine_with_limits(limits)
        .start_session(SessionId::new(30).unwrap());
    let timeout_error = session
        .navigate(
            NavigationRequest::get(server.url("/slow")),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert_eq!(timeout_error.class(), FailureClass::Timeout);

    let session = server.engine().start_session(SessionId::new(31).unwrap());
    let token = CancellationToken::new();
    let task_token = token.clone();
    let destination = server.url("/slow");
    let task = tokio::spawn(async move {
        session
            .navigate(NavigationRequest::get(destination), &task_token)
            .await
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    token.cancel();
    let cancellation_error = task.await.unwrap().unwrap_err();
    assert_eq!(cancellation_error.class(), FailureClass::Cancelled);
}

#[tokio::test]
async fn destination_policy_blocks_private_pivots_and_url_credentials() {
    let server = FixtureServer::spawn().await;
    let cancellation = CancellationToken::new();

    let public = NativeStaticEngine::new(NativeStaticConfig::default())
        .start_session(SessionId::new(40).unwrap());
    let loopback_error = public
        .navigate(NavigationRequest::get(server.url("/get")), &cancellation)
        .await
        .unwrap_err();
    assert!(matches!(
        loopback_error,
        OperationFailure::AuthorizationDenied {
            reason: AuthorizationReason::DestinationDenied,
            ..
        }
    ));

    let allowed = server.engine().start_session(SessionId::new(41).unwrap());
    let redirect_error = allowed
        .navigate(
            NavigationRequest::get(server.url("/private-redirect")),
            &cancellation,
        )
        .await
        .unwrap_err();
    assert_eq!(redirect_error.class(), FailureClass::AuthorizationDenied);

    let credential_url =
        AbsoluteUrl::new(format!("http://user:secret@127.0.0.1:{}/get", server.port)).unwrap();
    let credential_error = allowed
        .navigate(NavigationRequest::get(credential_url), &cancellation)
        .await
        .unwrap_err();
    assert_eq!(credential_error.class(), FailureClass::AuthorizationDenied);
    assert!(!format!("{credential_error:?}").contains("secret"));
}

#[tokio::test]
async fn downloads_never_overwrite_and_remove_oversized_partials() {
    let server = FixtureServer::spawn().await;
    let session = server.engine().start_session(SessionId::new(50).unwrap());
    let directory = TestDirectory::new();
    let policy = DownloadPolicy::new(directory.path(), 1024).unwrap();
    let filename = SafeFilename::new("artifact.bin").unwrap();
    let cancellation = CancellationToken::new();

    let result = session
        .download(
            DownloadRequest::new(server.url("/download"), filename.clone(), policy.clone()),
            &cancellation,
        )
        .await
        .unwrap();
    assert_eq!(result.bytes_written(), 16);
    assert_eq!(std::fs::read(result.path()).unwrap(), b"download-content");

    let overwrite = session
        .download(
            DownloadRequest::new(server.url("/download"), filename, policy),
            &cancellation,
        )
        .await
        .unwrap_err();
    assert_eq!(overwrite.class(), FailureClass::AuthorizationDenied);
    assert_eq!(std::fs::read(result.path()).unwrap(), b"download-content");

    let limited_policy = DownloadPolicy::new(directory.path(), 4).unwrap();
    let limited_name = SafeFilename::new("limited.bin").unwrap();
    let limited_path = directory.path().join(limited_name.as_str());
    let limited = session
        .download(
            DownloadRequest::new(server.url("/download"), limited_name, limited_policy),
            &cancellation,
        )
        .await
        .unwrap_err();
    assert!(matches!(
        limited,
        OperationFailure::ResourceLimit {
            resource: ResourceKind::DownloadBytes,
            ..
        }
    ));
    assert!(!limited_path.exists());

    let cancelled_name = SafeFilename::new("cancelled.bin").unwrap();
    let cancelled_path = directory.path().join(cancelled_name.as_str());
    let cancelled_policy = DownloadPolicy::new(directory.path(), 1024).unwrap();
    let token = CancellationToken::new();
    let task_token = token.clone();
    let slow_request = DownloadRequest::new(
        server.url("/slow-download"),
        cancelled_name,
        cancelled_policy,
    );
    let task = tokio::spawn(async move { session.download(slow_request, &task_token).await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    token.cancel();
    assert_eq!(
        task.await.unwrap().unwrap_err().class(),
        FailureClass::Cancelled
    );
    assert!(!cancelled_path.exists());
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "mawr-native-static-test-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if self
            .0
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("mawr-native-static-test-"))
        {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
