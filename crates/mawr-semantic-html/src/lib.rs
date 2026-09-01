//! Deterministic, bounded semantic extraction from static HTML.
//!
//! This crate implements a documented subset of HTML-AAM, WAI-ARIA 1.2 and
//! Accessible Name 1.1. It does not execute scripts or model CSS layout.

mod config;
mod decode;
mod dom;
mod extract;
mod model;
mod normalize;
mod roles;
mod state;
mod tree;

pub use config::{ConfigError, ExtractionLimits};
pub use model::{
    ExtractedRelationship, ExtractedSemanticUnit, ExtractionDiagnostics, ExtractionNotice,
    ExtractionNoticeKind, RoleOrigin, SemanticDocument, SourceNodeId,
};

use mawr_core::{AbsoluteUrl, OperationFailure, SessionId};
use mawr_native_static::DocumentInput;

#[derive(Debug, Clone, Copy)]
pub struct HtmlDocumentSource<'a> {
    session: SessionId,
    document_url: &'a AbsoluteUrl,
    content_type: Option<&'a str>,
    bytes: &'a [u8],
}

impl<'a> HtmlDocumentSource<'a> {
    #[must_use]
    pub const fn new(session: SessionId, document_url: &'a AbsoluteUrl, bytes: &'a [u8]) -> Self {
        Self {
            session,
            document_url,
            content_type: None,
            bytes,
        }
    }

    #[must_use]
    pub const fn with_content_type(mut self, content_type: Option<&'a str>) -> Self {
        self.content_type = content_type;
        self
    }
}

impl<'a> From<&'a DocumentInput> for HtmlDocumentSource<'a> {
    fn from(document: &'a DocumentInput) -> Self {
        Self {
            session: document.session(),
            document_url: document.final_url(),
            content_type: document.metadata().content_type(),
            bytes: document.body(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HtmlSemanticExtractor {
    limits: ExtractionLimits,
}

impl HtmlSemanticExtractor {
    #[must_use]
    pub const fn new(limits: ExtractionLimits) -> Self {
        Self { limits }
    }

    pub fn extract(&self, document: &DocumentInput) -> Result<SemanticDocument, OperationFailure> {
        self.extract_source(HtmlDocumentSource::from(document))
    }

    pub fn extract_source(
        &self,
        source: HtmlDocumentSource<'_>,
    ) -> Result<SemanticDocument, OperationFailure> {
        extract::extract(source, &self.limits)
    }
}
