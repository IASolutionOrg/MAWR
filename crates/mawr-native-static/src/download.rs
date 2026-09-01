use std::fmt;
use std::path::{Path, PathBuf};

use mawr_core::{AbsoluteUrl, SessionId};

use crate::{DownloadPolicy, SafeFilename, TransportDiagnostics};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadRequest {
    destination: AbsoluteUrl,
    filename: SafeFilename,
    policy: DownloadPolicy,
}

impl DownloadRequest {
    #[must_use]
    pub const fn new(
        destination: AbsoluteUrl,
        filename: SafeFilename,
        policy: DownloadPolicy,
    ) -> Self {
        Self {
            destination,
            filename,
            policy,
        }
    }

    #[must_use]
    pub const fn destination(&self) -> &AbsoluteUrl {
        &self.destination
    }

    pub(crate) const fn filename(&self) -> &SafeFilename {
        &self.filename
    }

    pub(crate) const fn policy(&self) -> &DownloadPolicy {
        &self.policy
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DownloadResult {
    session: SessionId,
    final_url: AbsoluteUrl,
    status: u16,
    path: PathBuf,
    bytes_written: u64,
    diagnostics: TransportDiagnostics,
}

impl DownloadResult {
    pub(crate) fn new(
        session: SessionId,
        final_url: AbsoluteUrl,
        status: u16,
        path: PathBuf,
        bytes_written: u64,
        diagnostics: TransportDiagnostics,
    ) -> Self {
        Self {
            session,
            final_url,
            status,
            path,
            bytes_written,
            diagnostics,
        }
    }

    #[must_use]
    pub const fn session(&self) -> SessionId {
        self.session
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
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub const fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    #[must_use]
    pub const fn diagnostics(&self) -> &TransportDiagnostics {
        &self.diagnostics
    }
}

impl fmt::Debug for DownloadResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DownloadResult")
            .field("session", &self.session)
            .field("final_url", &self.final_url)
            .field("status", &self.status)
            .field("path", &"<redacted>")
            .field("bytes_written", &self.bytes_written)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}
