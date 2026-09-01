use std::cmp;
use std::collections::BTreeSet;
use std::fmt;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::num::{NonZeroU32, NonZeroU64};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use mawr_core::{
    AbsoluteUrl, AuthorizationReason, Capability, CapabilityConstraint, CapabilityConstraints,
    CapabilityReport, CapabilityStatus, EngineFailureKind, EngineIdentity, EngineKind,
    NavigationFailureKind, OperationFailure, OperationKind, ProtocolFailureKind, ResourceKind,
    SessionId, UnsupportedReason,
};
use reqwest::header::{ACCEPT, CONTENT_LANGUAGE, CONTENT_TYPE, LOCATION};
use reqwest::{Client, Method, Response, StatusCode, Version};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::net::lookup_host;
use tokio::time::timeout;
use url::{Host, Url};

use crate::{
    CancellationToken, DocumentInput, DocumentMetadata, DownloadRequest, DownloadResult,
    FormMethod, HttpVersion, NativeStaticConfig, NavigationRequest, RedirectRecord, RequestMethod,
    TlsTrust, TransportDiagnostics, cookie::BoundedCookieJar,
};

const ENGINE_NAME: &str = "native-static";
const USER_AGENT: &str = concat!("MAWR/", env!("CARGO_PKG_VERSION"));
const ACCEPT_VALUE: &str =
    "text/html,application/xhtml+xml,application/octet-stream;q=0.8,*/*;q=0.5";
const MAX_METADATA_VALUE_BYTES: usize = 1_024;

#[derive(Debug, Clone)]
pub struct NativeStaticEngine {
    config: Arc<NativeStaticConfig>,
    identity: EngineIdentity,
    capabilities: CapabilityReport,
}

impl NativeStaticEngine {
    #[must_use]
    pub fn new(config: NativeStaticConfig) -> Self {
        let identity = EngineIdentity::new(
            ENGINE_NAME,
            env!("CARGO_PKG_VERSION"),
            EngineKind::NativeStatic,
        )
        .expect("static engine identity constants are valid");
        let capabilities = capability_report(&config, identity.clone());
        Self {
            config: Arc::new(config),
            identity,
            capabilities,
        }
    }

    #[must_use]
    pub const fn identity(&self) -> &EngineIdentity {
        &self.identity
    }

    #[must_use]
    pub const fn capabilities(&self) -> &CapabilityReport {
        &self.capabilities
    }

    #[must_use]
    pub fn start_session(&self, session: SessionId) -> StaticSession {
        StaticSession {
            session,
            config: Arc::clone(&self.config),
            identity: self.identity.clone(),
            capabilities: self.capabilities.clone(),
            cookies: Arc::new(BoundedCookieJar::new(
                self.config.limits().max_cookie_bytes(),
            )),
        }
    }
}

#[derive(Clone)]
pub struct StaticSession {
    session: SessionId,
    config: Arc<NativeStaticConfig>,
    identity: EngineIdentity,
    capabilities: CapabilityReport,
    cookies: Arc<BoundedCookieJar>,
}

impl StaticSession {
    #[must_use]
    pub const fn id(&self) -> SessionId {
        self.session
    }

    #[must_use]
    pub const fn engine_identity(&self) -> &EngineIdentity {
        &self.identity
    }

    #[must_use]
    pub const fn capabilities(&self) -> &CapabilityReport {
        &self.capabilities
    }

    pub async fn navigate(
        &self,
        request: NavigationRequest,
        cancellation: &CancellationToken,
    ) -> Result<DocumentInput, OperationFailure> {
        let operation = OperationKind::Navigate;
        let total_timeout = self.config.limits().total_timeout();
        self.with_controls(operation, cancellation, async {
            let started = Instant::now();
            let requested_url = request.destination().clone();
            let prepared = self.prepare_request(request)?;
            let final_response = self.fetch_final(prepared, operation).await?;
            let FinalResponse {
                final_url,
                status,
                version,
                metadata,
                response,
                redirects,
                request_count,
            } = final_response;
            let body = read_bounded_body(
                response,
                self.config.limits().max_response_bytes(),
                ResourceKind::ResponseBytes,
                operation,
                total_timeout,
                &self.identity,
            )
            .await?;
            let diagnostics = TransportDiagnostics::new(
                request_count,
                redirects.len() as u32,
                body.len() as u64,
                started.elapsed(),
            );
            Ok(DocumentInput::new(
                self.session,
                requested_url,
                final_url,
                status,
                version,
                metadata,
                body,
                redirects,
                diagnostics,
            ))
        })
        .await
    }

    pub async fn download(
        &self,
        request: DownloadRequest,
        cancellation: &CancellationToken,
    ) -> Result<DownloadResult, OperationFailure> {
        let operation = OperationKind::Download;
        let total_timeout = self.config.limits().total_timeout();
        self.with_controls(operation, cancellation, async {
            let started = Instant::now();
            let prepared =
                self.prepare_request(NavigationRequest::get(request.destination().clone()))?;
            let final_response = self.fetch_final(prepared, operation).await?;
            if !(200..300).contains(&final_response.status) {
                return Err(OperationFailure::NavigationFailure(
                    NavigationFailureKind::Response,
                ));
            }

            let current_root = tokio::fs::canonicalize(request.policy().root())
                .await
                .map_err(|_| download_io_failure(&self.identity))?;
            if &current_root != request.policy().root() || !current_root.is_dir() {
                return Err(OperationFailure::AuthorizationDenied {
                    operation,
                    reason: AuthorizationReason::PolicyDenied,
                });
            }
            let destination = current_root.join(request.filename().as_str());
            let mut cleanup = PartialDownload::new(destination.clone());
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)
                .await
                .map_err(|_| OperationFailure::AuthorizationDenied {
                    operation,
                    reason: AuthorizationReason::PolicyDenied,
                })?;
            cleanup.activate();

            let limit = cmp::min(
                self.config.limits().max_download_bytes(),
                request.policy().max_bytes(),
            );
            let mut response = final_response.response;
            if response
                .content_length()
                .is_some_and(|length| length > limit)
            {
                return Err(resource_limit(ResourceKind::DownloadBytes, limit));
            }
            let mut written = 0_u64;
            while let Some(chunk) = response.chunk().await.map_err(|error| {
                map_reqwest_error(error, operation, total_timeout, &self.identity)
            })? {
                written = written
                    .checked_add(chunk.len() as u64)
                    .ok_or_else(|| resource_limit(ResourceKind::DownloadBytes, limit))?;
                if written > limit {
                    return Err(resource_limit(ResourceKind::DownloadBytes, limit));
                }
                file.write_all(&chunk)
                    .await
                    .map_err(|_| download_io_failure(&self.identity))?;
            }
            file.flush()
                .await
                .map_err(|_| download_io_failure(&self.identity))?;
            cleanup.disarm();

            let diagnostics = TransportDiagnostics::new(
                final_response.request_count,
                final_response.redirects.len() as u32,
                written,
                started.elapsed(),
            );
            Ok(DownloadResult::new(
                self.session,
                final_response.final_url,
                final_response.status,
                destination,
                written,
                diagnostics,
            ))
        })
        .await
    }

    #[must_use]
    pub fn unsupported(&self, capability: Capability) -> OperationFailure {
        OperationFailure::UnsupportedCapability {
            capability,
            engine: self.identity.clone(),
            reason: UnsupportedReason::EngineLimitation,
        }
    }

    async fn with_controls<T, F>(
        &self,
        operation: OperationKind,
        cancellation: &CancellationToken,
        future: F,
    ) -> Result<T, OperationFailure>
    where
        F: Future<Output = Result<T, OperationFailure>>,
    {
        let limit = self.config.limits().total_timeout();
        tokio::select! {
            () = cancellation.cancelled() => Err(OperationFailure::Cancelled { operation }),
            result = timeout(limit, future) => {
                result.unwrap_or_else(|_| Err(timeout_failure(operation, limit)))
            }
        }
    }

    fn prepare_request(
        &self,
        request: NavigationRequest,
    ) -> Result<PreparedRequest, OperationFailure> {
        let mut url = Url::parse(request.destination().as_str()).map_err(|_| {
            OperationFailure::NavigationFailure(NavigationFailureKind::InvalidDestination)
        })?;
        url.set_fragment(None);
        let mut body = None;
        if let Some((form_method, submission)) = request.form() {
            let mut serializer = url::form_urlencoded::Serializer::new(String::new());
            for field in submission.fields() {
                serializer.append_pair(field.name(), field.value());
            }
            let encoded = serializer.finish();
            if encoded.len() as u64 > self.config.limits().max_form_bytes() {
                return Err(resource_limit(
                    ResourceKind::FormBytes,
                    self.config.limits().max_form_bytes(),
                ));
            }
            match form_method {
                FormMethod::Get => {
                    let query = match url.query() {
                        Some(existing) if !existing.is_empty() && !encoded.is_empty() => {
                            format!("{existing}&{encoded}")
                        }
                        Some(existing) if !existing.is_empty() => existing.to_owned(),
                        _ => encoded,
                    };
                    url.set_query((!query.is_empty()).then_some(&query));
                }
                FormMethod::Post => body = Some(encoded.into_bytes()),
            }
        }
        let method = match request.method() {
            RequestMethod::Get => Method::GET,
            RequestMethod::Head => Method::HEAD,
            RequestMethod::Post => Method::POST,
        };
        AbsoluteUrl::new(url.as_str()).map_err(OperationFailure::InvalidInput)?;
        Ok(PreparedRequest { method, url, body })
    }

    async fn fetch_final(
        &self,
        mut prepared: PreparedRequest,
        operation: OperationKind,
    ) -> Result<FinalResponse, OperationFailure> {
        let mut redirects = Vec::new();
        let mut visited = BTreeSet::from([prepared.url.as_str().to_owned()]);
        let mut request_count = 0_u32;

        loop {
            if self.cookies.is_exceeded() {
                return Err(resource_limit(
                    ResourceKind::SessionCookies,
                    self.config.limits().max_cookie_bytes(),
                ));
            }
            let resolved = self.authorize_destination(&prepared.url, operation).await?;
            let client = self.build_client(&resolved)?;
            let mut request = client
                .request(prepared.method.clone(), prepared.url.clone())
                .header(ACCEPT, ACCEPT_VALUE);
            if let Some(body) = &prepared.body {
                request = request
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(body.clone());
            }
            let response = request.send().await.map_err(|error| {
                map_reqwest_error(
                    error,
                    operation,
                    self.config.limits().total_timeout(),
                    &self.identity,
                )
            })?;
            if self.cookies.is_exceeded() {
                return Err(resource_limit(
                    ResourceKind::SessionCookies,
                    self.config.limits().max_cookie_bytes(),
                ));
            }
            request_count = request_count.saturating_add(1);
            verify_remote_address(&response, &resolved, operation)?;
            validate_headers(
                &response,
                self.config.limits().max_header_count(),
                self.config.limits().max_header_bytes(),
            )?;

            if is_followed_redirect(response.status()) {
                if redirects.len() >= self.config.limits().max_redirects() {
                    return Err(OperationFailure::NavigationFailure(
                        NavigationFailureKind::TooManyRedirects,
                    ));
                }
                let location = response.headers().get(LOCATION).ok_or({
                    OperationFailure::NavigationFailure(
                        NavigationFailureKind::MissingRedirectLocation,
                    )
                })?;
                let location = location.to_str().map_err(|_| {
                    OperationFailure::ProtocolFailure(ProtocolFailureKind::InvalidMessage)
                })?;
                let mut next = prepared.url.join(location).map_err(|_| {
                    OperationFailure::NavigationFailure(NavigationFailureKind::InvalidDestination)
                })?;
                next.set_fragment(None);
                if !visited.insert(next.as_str().to_owned()) {
                    return Err(OperationFailure::NavigationFailure(
                        NavigationFailureKind::RedirectLoop,
                    ));
                }
                let from = AbsoluteUrl::new(prepared.url.as_str())
                    .map_err(OperationFailure::InvalidInput)?;
                let to = AbsoluteUrl::new(next.as_str()).map_err(OperationFailure::InvalidInput)?;
                redirects.push(RedirectRecord::new(response.status().as_u16(), from, to));
                rewrite_redirect_request(response.status(), &mut prepared);
                prepared.url = next;
                continue;
            }

            let final_url =
                AbsoluteUrl::new(prepared.url.as_str()).map_err(OperationFailure::InvalidInput)?;
            let status = response.status().as_u16();
            let version = http_version(response.version())?;
            let metadata = metadata(response.headers())?;
            return Ok(FinalResponse {
                final_url,
                status,
                version,
                metadata,
                response,
                redirects,
                request_count,
            });
        }
    }

    async fn authorize_destination(
        &self,
        url: &Url,
        operation: OperationKind,
    ) -> Result<ResolvedDestination, OperationFailure> {
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(destination_denied(operation));
        }
        let port = url
            .port_or_known_default()
            .ok_or_else(|| destination_denied(operation))?;
        if !self.config.destination_policy().allows_port(port) {
            return Err(destination_denied(operation));
        }
        let host = url.host().ok_or_else(|| destination_denied(operation))?;
        let (domain, mut addresses) = match host {
            Host::Domain(domain) => {
                let resolved = timeout(
                    self.config.limits().connect_timeout(),
                    lookup_host((domain, port)),
                )
                .await
                .map_err(|_| timeout_failure(operation, self.config.limits().connect_timeout()))?
                .map_err(|_| OperationFailure::NavigationFailure(NavigationFailureKind::Dns))?
                .collect::<Vec<_>>();
                (Some(domain.to_owned()), resolved)
            }
            Host::Ipv4(address) => (None, vec![SocketAddr::new(IpAddr::V4(address), port)]),
            Host::Ipv6(address) => (None, vec![SocketAddr::new(IpAddr::V6(address), port)]),
        };
        addresses.sort_unstable();
        addresses.dedup();
        if addresses.is_empty() {
            return Err(OperationFailure::NavigationFailure(
                NavigationFailureKind::Dns,
            ));
        }
        if addresses.len() > self.config.limits().max_dns_addresses() {
            return Err(resource_limit(
                ResourceKind::DnsAddresses,
                self.config.limits().max_dns_addresses() as u64,
            ));
        }
        if addresses.iter().any(|address| {
            !self
                .config
                .destination_policy()
                .allows_address(address.ip())
        }) {
            return Err(destination_denied(operation));
        }
        Ok(ResolvedDestination { domain, addresses })
    }

    fn build_client(&self, destination: &ResolvedDestination) -> Result<Client, OperationFailure> {
        let limits = self.config.limits();
        let mut builder = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .referer(false)
            .user_agent(USER_AGENT)
            .connect_timeout(limits.connect_timeout())
            .read_timeout(limits.read_timeout())
            .timeout(limits.total_timeout())
            .pool_max_idle_per_host(0)
            .http1_only()
            .tls_backend_rustls()
            .tls_version_min(reqwest::tls::Version::TLS_1_2)
            .cookie_provider(Arc::clone(&self.cookies))
            .gzip(true)
            .brotli(true)
            .deflate(true);
        if let Some(domain) = &destination.domain {
            builder = builder.resolve_to_addrs(domain, &destination.addresses);
        }
        if let TlsTrust::Only(certificates) = self.config.tls_trust() {
            builder = builder.tls_certs_only(certificates.iter().map(|cert| cert.to_reqwest()));
        }
        builder
            .build()
            .map_err(|_| OperationFailure::EngineFailure {
                engine: self.identity.clone(),
                kind: EngineFailureKind::Startup,
            })
    }
}

impl fmt::Debug for StaticSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StaticSession")
            .field("session", &self.session)
            .field("identity", &self.identity)
            .field("capabilities", &self.capabilities)
            .field("cookies", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone)]
struct PreparedRequest {
    method: Method,
    url: Url,
    body: Option<Vec<u8>>,
}

struct FinalResponse {
    final_url: AbsoluteUrl,
    status: u16,
    version: HttpVersion,
    metadata: DocumentMetadata,
    response: Response,
    redirects: Vec<RedirectRecord>,
    request_count: u32,
}

#[derive(Debug)]
struct ResolvedDestination {
    domain: Option<String>,
    addresses: Vec<SocketAddr>,
}

struct PartialDownload {
    path: PathBuf,
    active: bool,
}

impl PartialDownload {
    const fn new(path: PathBuf) -> Self {
        Self {
            path,
            active: false,
        }
    }

    const fn activate(&mut self) {
        self.active = true;
    }

    const fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for PartialDownload {
    fn drop(&mut self) {
        if self.active {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn capability_report(config: &NativeStaticConfig, identity: EngineIdentity) -> CapabilityReport {
    let response_limit = CapabilityConstraints::new(CapabilityConstraint::MaxBytes(
        NonZeroU64::new(config.limits().max_response_bytes()).expect("limit is non-zero"),
    ));
    let redirect_limit = CapabilityConstraints::new(CapabilityConstraint::MaxOperations(
        NonZeroU32::new(config.limits().max_redirects() as u32).expect("limit is non-zero"),
    ));
    let download_limit = CapabilityConstraints::new(CapabilityConstraint::MaxBytes(
        NonZeroU64::new(config.limits().max_download_bytes()).expect("limit is non-zero"),
    ));
    CapabilityReport::unsupported_all(identity, UnsupportedReason::NotImplemented)
        .with(
            Capability::Navigation,
            CapabilityStatus::Limited(response_limit.clone()),
        )
        .with(
            Capability::Http,
            CapabilityStatus::Limited(response_limit.clone()),
        )
        .with(Capability::Https, CapabilityStatus::Limited(response_limit))
        .with(
            Capability::Redirects,
            CapabilityStatus::Limited(redirect_limit),
        )
        .with(Capability::FormGet, CapabilityStatus::Supported)
        .with(Capability::FormPost, CapabilityStatus::Supported)
        .with(
            Capability::Downloads,
            CapabilityStatus::Limited(download_limit),
        )
        .with(
            Capability::SessionCookies,
            CapabilityStatus::Limited(CapabilityConstraints::new(CapabilityConstraint::MaxBytes(
                NonZeroU64::new(config.limits().max_cookie_bytes()).expect("limit is non-zero"),
            ))),
        )
}

fn validate_headers(
    response: &Response,
    max_count: usize,
    max_bytes: u64,
) -> Result<(), OperationFailure> {
    if response.headers().len() > max_count {
        return Err(resource_limit(
            ResourceKind::ResponseHeaders,
            max_count as u64,
        ));
    }
    let bytes = response
        .headers()
        .iter()
        .try_fold(0_u64, |total, (name, value)| {
            total
                .checked_add(name.as_str().len() as u64)
                .and_then(|total| total.checked_add(value.as_bytes().len() as u64))
        });
    if bytes.is_none_or(|bytes| bytes > max_bytes) {
        return Err(resource_limit(ResourceKind::ResponseHeaders, max_bytes));
    }
    Ok(())
}

fn metadata(headers: &reqwest::header::HeaderMap) -> Result<DocumentMetadata, OperationFailure> {
    Ok(DocumentMetadata::new(
        metadata_header(headers, CONTENT_TYPE)?,
        metadata_header(headers, CONTENT_LANGUAGE)?,
    ))
}

fn metadata_header(
    headers: &reqwest::header::HeaderMap,
    name: reqwest::header::HeaderName,
) -> Result<Option<String>, OperationFailure> {
    let Some(value) = headers.get(name) else {
        return Ok(None);
    };
    if value.as_bytes().len() > MAX_METADATA_VALUE_BYTES {
        return Err(resource_limit(
            ResourceKind::ResponseHeaders,
            MAX_METADATA_VALUE_BYTES as u64,
        ));
    }
    value
        .to_str()
        .map(|value| Some(value.to_owned()))
        .map_err(|_| OperationFailure::ProtocolFailure(ProtocolFailureKind::InvalidMessage))
}

async fn read_bounded_body(
    mut response: Response,
    limit: u64,
    resource: ResourceKind,
    operation: OperationKind,
    timeout_limit: Duration,
    identity: &EngineIdentity,
) -> Result<Vec<u8>, OperationFailure> {
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err(resource_limit(resource, limit));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| map_reqwest_error(error, operation, timeout_limit, identity))?
    {
        let length = body
            .len()
            .checked_add(chunk.len())
            .ok_or_else(|| resource_limit(resource, limit))?;
        if length as u64 > limit {
            return Err(resource_limit(resource, limit));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn verify_remote_address(
    response: &Response,
    destination: &ResolvedDestination,
    operation: OperationKind,
) -> Result<(), OperationFailure> {
    let remote = response
        .remote_addr()
        .ok_or_else(|| destination_denied(operation))?;
    if destination.addresses.contains(&remote) {
        Ok(())
    } else {
        Err(destination_denied(operation))
    }
}

fn rewrite_redirect_request(status: StatusCode, prepared: &mut PreparedRequest) {
    let rewrite_to_get = status == StatusCode::SEE_OTHER && prepared.method != Method::HEAD
        || matches!(status, StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND)
            && prepared.method == Method::POST;
    if rewrite_to_get {
        prepared.method = Method::GET;
        prepared.body = None;
    }
}

fn is_followed_redirect(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT
    )
}

fn http_version(version: Version) -> Result<HttpVersion, OperationFailure> {
    match version {
        Version::HTTP_10 => Ok(HttpVersion::Http10),
        Version::HTTP_11 => Ok(HttpVersion::Http11),
        _ => Err(OperationFailure::ProtocolFailure(
            ProtocolFailureKind::InvalidMessage,
        )),
    }
}

fn map_reqwest_error(
    error: reqwest::Error,
    operation: OperationKind,
    timeout_limit: Duration,
    identity: &EngineIdentity,
) -> OperationFailure {
    if error.is_timeout() {
        timeout_failure(operation, timeout_limit)
    } else if error.is_builder() {
        OperationFailure::NavigationFailure(NavigationFailureKind::InvalidDestination)
    } else if error.is_connect() {
        if error.url().is_some_and(|url| url.scheme() == "https") {
            OperationFailure::NavigationFailure(NavigationFailureKind::SecureConnection)
        } else {
            OperationFailure::NavigationFailure(NavigationFailureKind::Connection)
        }
    } else if error.is_redirect() {
        OperationFailure::NavigationFailure(NavigationFailureKind::RedirectLoop)
    } else if error.is_request() || error.is_body() || error.is_decode() {
        OperationFailure::NavigationFailure(NavigationFailureKind::Response)
    } else {
        OperationFailure::EngineFailure {
            engine: identity.clone(),
            kind: EngineFailureKind::Execution,
        }
    }
}

fn resource_limit(resource: ResourceKind, configured_limit: u64) -> OperationFailure {
    OperationFailure::ResourceLimit {
        resource,
        configured_limit: NonZeroU64::new(configured_limit)
            .expect("configured limits are non-zero"),
    }
}

fn destination_denied(operation: OperationKind) -> OperationFailure {
    OperationFailure::AuthorizationDenied {
        operation,
        reason: AuthorizationReason::DestinationDenied,
    }
}

fn timeout_failure(operation: OperationKind, limit: Duration) -> OperationFailure {
    let millis = u64::try_from(limit.as_millis()).unwrap_or(u64::MAX).max(1);
    OperationFailure::Timeout {
        operation,
        limit_millis: NonZeroU64::new(millis).expect("milliseconds are clamped to non-zero"),
    }
}

fn download_io_failure(identity: &EngineIdentity) -> OperationFailure {
    OperationFailure::EngineFailure {
        engine: identity.clone(),
        kind: EngineFailureKind::Execution,
    }
}
