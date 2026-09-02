#![forbid(unsafe_code)]

mod config;
mod matcher;
mod model;
mod store;

pub use config::{ConfigError, StateStoreConfig};
pub use model::{
    ReferenceAssignment, ReferenceAssignmentReason, ReferenceLoss, ReferenceLossReason,
    StableRelationship, StateDiagnostics, StateUpdate, StoredSemanticUnit, StoredState,
};
pub use store::SemanticStateStore;
