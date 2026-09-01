use std::fmt;
use std::time::Duration;

use mawr_core::{
    AbsoluteUrl, Measurement, MeasurementKind, MeasurementSet, MeasurementSource, SessionId,
    UnavailableReason,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HttpVersion {
    Http10,
    Http11,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentMetadata {
    content_type: Option<String>,
    content_language: Option<String>,
}

impl DocumentMetadata {
    pub(crate) fn new(content_type: Option<String>, content_language: Option<String>) -> Self {
        Self {
            content_type,
            content_language,
        }
    }

    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    #[must_use]
    pub fn content_language(&self) -> Option<&str> {
        self.content_language.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectRecord {
    status: u16,
    from: AbsoluteUrl,
    to: AbsoluteUrl,
}

impl RedirectRecord {
    pub(crate) const fn new(status: u16, from: AbsoluteUrl, to: AbsoluteUrl) -> Self {
        Self { status, from, to }
    }

    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    #[must_use]
    pub const fn from(&self) -> &AbsoluteUrl {
        &self.from
    }

    #[must_use]
    pub const fn to(&self) -> &AbsoluteUrl {
        &self.to
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportDiagnostics {
    request_count: u32,
    redirect_count: u32,
    decoded_body_bytes: u64,
    measurements: MeasurementSet,
}

impl TransportDiagnostics {
    pub(crate) fn new(
        request_count: u32,
        redirect_count: u32,
        decoded_body_bytes: u64,
        elapsed: Duration,
    ) -> Self {
        let elapsed_micros = u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX);
        let measurements = MeasurementSet::default()
            .with(
                MeasurementKind::LatencyMicros,
                Measurement::Exact {
                    value: elapsed_micros,
                    source: MeasurementSource::RuntimeCounter,
                },
            )
            .with(
                MeasurementKind::NetworkBytes,
                Measurement::Unavailable(UnavailableReason::SourceMissing),
            )
            .with(
                MeasurementKind::CpuMicros,
                Measurement::Unavailable(UnavailableReason::NotMeasured),
            )
            .with(
                MeasurementKind::Retries,
                Measurement::Exact {
                    value: 0,
                    source: MeasurementSource::RuntimeCounter,
                },
            );
        Self {
            request_count,
            redirect_count,
            decoded_body_bytes,
            measurements,
        }
    }

    #[must_use]
    pub const fn request_count(&self) -> u32 {
        self.request_count
    }

    #[must_use]
    pub const fn redirect_count(&self) -> u32 {
        self.redirect_count
    }

    #[must_use]
    pub const fn decoded_body_bytes(&self) -> u64 {
        self.decoded_body_bytes
    }

    #[must_use]
    pub const fn measurements(&self) -> &MeasurementSet {
        &self.measurements
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DocumentInput {
    session: SessionId,
    requested_url: AbsoluteUrl,
    final_url: AbsoluteUrl,
    status: u16,
    version: HttpVersion,
    metadata: DocumentMetadata,
    body: Vec<u8>,
    redirects: Vec<RedirectRecord>,
    diagnostics: TransportDiagnostics,
}

impl DocumentInput {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        session: SessionId,
        requested_url: AbsoluteUrl,
        final_url: AbsoluteUrl,
        status: u16,
        version: HttpVersion,
        metadata: DocumentMetadata,
        body: Vec<u8>,
        redirects: Vec<RedirectRecord>,
        diagnostics: TransportDiagnostics,
    ) -> Self {
        Self {
            session,
            requested_url,
            final_url,
            status,
            version,
            metadata,
            body,
            redirects,
            diagnostics,
        }
    }

    #[must_use]
    pub const fn session(&self) -> SessionId {
        self.session
    }

    #[must_use]
    pub const fn requested_url(&self) -> &AbsoluteUrl {
        &self.requested_url
    }

    #[must_use]
    pub const fn final_url(&self) -> &AbsoluteUrl {
        &self.final_url
    }

    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    #[must_use]
    pub const fn version(&self) -> HttpVersion {
        self.version
    }

    #[must_use]
    pub const fn metadata(&self) -> &DocumentMetadata {
        &self.metadata
    }

    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    #[must_use]
    pub fn redirects(&self) -> &[RedirectRecord] {
        &self.redirects
    }

    #[must_use]
    pub const fn diagnostics(&self) -> &TransportDiagnostics {
        &self.diagnostics
    }
}

impl fmt::Debug for DocumentInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DocumentInput")
            .field("session", &self.session)
            .field("requested_url", &self.requested_url)
            .field("final_url", &self.final_url)
            .field("status", &self.status)
            .field("version", &self.version)
            .field("metadata", &self.metadata)
            .field("body_bytes", &self.body.len())
            .field("redirects", &self.redirects)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}
