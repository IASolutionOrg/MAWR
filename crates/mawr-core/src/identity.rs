use crate::{AbsoluteUrl, NonEmptyText, PageId, ValidationError};

const MAX_ENGINE_NAME_BYTES: usize = 64;
const MAX_ENGINE_VERSION_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EngineKind {
    NativeStatic,
    ExternalAdapter,
}

/// MAWR-owned engine identity without vendor protocol types.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EngineIdentity {
    name: NonEmptyText<MAX_ENGINE_NAME_BYTES>,
    version: NonEmptyText<MAX_ENGINE_VERSION_BYTES>,
    kind: EngineKind,
}

impl EngineIdentity {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        kind: EngineKind,
    ) -> Result<Self, ValidationError> {
        Ok(Self {
            name: NonEmptyText::new(name, "engine_name")?,
            version: NonEmptyText::new(version, "engine_version")?,
            kind,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    #[must_use]
    pub fn version(&self) -> &str {
        self.version.as_str()
    }

    #[must_use]
    pub const fn kind(&self) -> EngineKind {
        self.kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageIdentity {
    id: PageId,
    url: AbsoluteUrl,
}

impl PageIdentity {
    #[must_use]
    pub const fn new(id: PageId, url: AbsoluteUrl) -> Self {
        Self { id, url }
    }

    #[must_use]
    pub const fn id(&self) -> PageId {
        self.id
    }

    #[must_use]
    pub const fn url(&self) -> &AbsoluteUrl {
        &self.url
    }
}
