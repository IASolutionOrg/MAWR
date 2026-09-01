//! Native asynchronous HTTP(S) transport for MAWR's static engine.
//!
//! This crate acquires bounded document bytes without parsing HTML or
//! depending on Chromium, an external engine, a CLI, or an encoding layer.

mod cancel;
mod config;
mod cookie;
mod download;
mod engine;
mod request;
mod response;

pub use cancel::CancellationToken;
pub use config::{
    ConfigError, DerCertificate, DestinationPolicy, DownloadPolicy, NativeStaticConfig,
    SafeFilename, TlsTrust, TransportLimits,
};
pub use download::{DownloadRequest, DownloadResult};
pub use engine::{NativeStaticEngine, StaticSession};
pub use request::{FormField, FormMethod, FormSubmission, NavigationRequest, RequestMethod};
pub use response::{
    DocumentInput, DocumentMetadata, HttpVersion, RedirectRecord, TransportDiagnostics,
};
