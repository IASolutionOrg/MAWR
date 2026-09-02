# Complete and incremental observations: implemented M5/M9 boundary

`mawr-observation` builds either a typed complete view of the current `mawr-state` snapshot or a bounded semantic delta from a retained same-page base. Complete observations remain the unfiltered diagnostic/reference mode required before relevance selection. Incremental observations preserve only meaningful changes and are reconstructed against their exact base. The crate does not rank, tokenize, encode, or expose a CLI/MCP surface.

## Complete semantic units

M5 extends the internal core unit so conversion from M3/M4 is lossless for task-relevant state. Each observation unit retains:

- stable reference and semantic parent when the parent is itself an emitted unit;
- role and provenance;
- tri-state known/unknown/not-applicable name and description;
- semantic value, including redacted and explicitly unknown forms;
- disabled, checked, selected, expanded, required, and invalid state;
- relationships translated to stable references;
- supported action affordances;
- tri-state HTTP(S) destination.

Author HTML IDs and per-parse `SourceNodeId` values remain local identity/provenance inputs and are not exposed as agent selectors. Relationships that target non-semantic source nodes are counted in build diagnostics rather than converted into invented targets; their name/description contribution is already present in the semantic properties.

## Request and basis behavior

`ObservationRequest` continues to validate bounded non-empty goals, bounded token budgets, session ownership, and `since_state`. Full mode deliberately does not use the goal or token budget for selection: its diagnostics mark each supplied input as deferred, and every unit remains present. The implemented [M6 selector](RELEVANCE.md) consumes that same request alongside the immutable full observation.

The builder classifies state history and computes changes only when the requested base is safe:

- the first state without a base is `Full(Initial)`;
- a later state without a base is `Full(NoBaseRequested)`;
- a retained same-page base is `Incremental` with typed `Changes::Computed`;
- an evicted or unknown base returns the current full state with `Reset(BaseEvicted|BaseUnavailable)`;
- a retained base across an explicit reset or page-identity boundary returns the recorded reset reason, including navigation or ambiguous identity;
- a delta that exceeds the configured change bound returns the current full state with `Reset(DiffTooLarge)`.

An empty store returns structured engine `StateUnavailable`. Request/store session mismatch, capability/engine mismatch, and semantic conversion invariant failures return structured invalid input. No state is mutated while observing.

## Summary, omissions, bounds, and metrics

The summary is the bounded document title or the deterministic fallback `Untitled page`; page identity separately carries the final URL. M3 bounds titles to 512 UTF-8 bytes, below the core 1,024-byte summary limit.

Full and incremental builder output has an all-zero `OmissionSummary`; selection remains a separate M6 operation. The default observation and change limits are each 250,000 entries and callers may configure either validated core `CollectionLimit`. Full units and emitted delta units are canonically sorted by reference, while `semantic_order` preserves document order for reconstruction and reorder detection.

The measurement envelope records exact observation-construction wall latency from a runtime counter. Observation tokens, CPU, and peak memory remain explicitly unavailable. Separate diagnostics record full and emitted unit counts, source input bytes, relationship counts, full and emitted logical-content bytes, latency, deferred request inputs, and optional diff counts/timing. Logical content bytes are typed-content measurements, not serialized size or a token estimate.

The fixed all-semantics, empty, error, retained/evicted/navigation, reconstruction, action-validation, and generated-transition cases provide correctness and construction-size/time diagnostics. They do not constitute a task-efficiency or process-memory claim. See [DIFFS.md](DIFFS.md) for the exact incremental contract.

## Dependency boundary

`mawr-observation` is private at version `0.0.0`. Its only direct runtime dependencies are `mawr-core` and `mawr-state`. Repository verification rejects direct encoding, subprocess, or additional runtime dependencies so M10 transport decisions cannot leak into observation or diff construction.
