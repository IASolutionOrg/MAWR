use std::array;

use crate::NonEmptyText;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MeasurementSource {
    ProviderReported,
    LocalTokenizer,
    RuntimeCounter,
    OperatingSystem,
    BenchmarkHarness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnavailableReason {
    NotMeasured,
    Unsupported,
    PermissionDenied,
    SourceMissing,
    MeasurementFailed,
}

/// A value whose epistemic quality cannot be lost or represented ambiguously.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Measurement<T> {
    Exact { value: T, source: MeasurementSource },
    Estimated { value: T, method: NonEmptyText<128> },
    Unavailable(UnavailableReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum MeasurementKind {
    ObservationTokens,
    InputTokens,
    CachedInputTokens,
    OutputTokens,
    ReasoningTokens,
    LatencyMicros,
    CpuMicros,
    PeakMemoryBytes,
    NetworkBytes,
    ModelCalls,
    Retries,
}

impl MeasurementKind {
    pub const COUNT: usize = 11;
    pub const ALL: [Self; Self::COUNT] = [
        Self::ObservationTokens,
        Self::InputTokens,
        Self::CachedInputTokens,
        Self::OutputTokens,
        Self::ReasoningTokens,
        Self::LatencyMicros,
        Self::CpuMicros,
        Self::PeakMemoryBytes,
        Self::NetworkBytes,
        Self::ModelCalls,
        Self::Retries,
    ];

    const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementSet {
    values: [Measurement<u64>; MeasurementKind::COUNT],
}

impl MeasurementSet {
    #[must_use]
    pub fn unavailable_all(reason: UnavailableReason) -> Self {
        Self {
            values: array::from_fn(|_| Measurement::Unavailable(reason)),
        }
    }

    #[must_use]
    pub fn with(mut self, kind: MeasurementKind, value: Measurement<u64>) -> Self {
        self.values[kind.index()] = value;
        self
    }

    #[must_use]
    pub const fn get(&self, kind: MeasurementKind) -> &Measurement<u64> {
        &self.values[kind.index()]
    }

    pub fn iter(&self) -> impl Iterator<Item = (MeasurementKind, &Measurement<u64>)> {
        MeasurementKind::ALL
            .into_iter()
            .map(|kind| (kind, self.get(kind)))
    }
}

impl Default for MeasurementSet {
    fn default() -> Self {
        Self::unavailable_all(UnavailableReason::NotMeasured)
    }
}

#[cfg(test)]
mod tests {
    use super::{Measurement, MeasurementKind, MeasurementSet, UnavailableReason};

    #[test]
    fn missing_metrics_are_never_implicit_zeroes() {
        let metrics = MeasurementSet::default();
        assert_eq!(
            metrics.get(MeasurementKind::NetworkBytes),
            &Measurement::Unavailable(UnavailableReason::NotMeasured)
        );
        assert_eq!(metrics.iter().count(), MeasurementKind::COUNT);
    }
}
