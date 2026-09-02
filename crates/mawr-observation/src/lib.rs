#![forbid(unsafe_code)]

mod builder;
mod diff;

pub use builder::{
    BuiltObservation, FullObservationBuilder, FullObservationConfig, ObservationBuildDiagnostics,
};
pub use diff::{SemanticDiffDiagnostics, SemanticSnapshot};
