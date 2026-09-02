# Complete observations: implemented M5 boundary

`mawr-observation` builds a typed, complete view of the current `mawr-state` snapshot. It is the unfiltered diagnostic/reference mode required before relevance selection: every semantic unit is preserved, sorted by stable `ElementRef`, and returned with state, page, engine, capability, basis, omission, and measurement metadata. It does not rank, tokenize, pack, diff, encode, or expose a CLI/MCP surface.

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

The builder classifies state history without pretending that M9 semantic diffs exist:

- the first state without a base is `Full(Initial)`;
- a later state without a base is `Full(NoBaseRequested)`;
- a retained same-page base is `Incremental` with `Changes::NotComputed` while the current full state is still returned;
- an evicted or unknown base returns the current full state with `Reset(BaseEvicted|BaseUnavailable)`;
- a retained base across a page-identity boundary returns `Reset(NavigationBoundary)`.

An empty store returns structured engine `StateUnavailable`. Request/store session mismatch, capability/engine mismatch, and semantic conversion invariant failures return structured invalid input. No state is mutated while observing.

## Summary, omissions, bounds, and metrics

The summary is the bounded document title or the deterministic fallback `Untitled page`; page identity separately carries the final URL. M3 bounds titles to 512 UTF-8 bytes, below the core 1,024-byte summary limit.

Full mode has an all-zero `OmissionSummary`: it either includes every semantic unit or fails with `ResourceLimit`. The default observation limit is 250,000 units and callers may select any validated core `CollectionLimit`. Units are collected and sorted once, avoiding repeated ordered-vector insertion for large pages.

The measurement envelope records exact observation-construction wall latency from a runtime counter. Observation tokens, CPU, and peak memory remain explicitly unavailable. Separate diagnostics record source input bytes, unit and relationship counts, unresolved relationship count, exact logical content bytes, latency, and whether goal/token-budget inputs were deferred. Logical content bytes count summary, page URL, known semantic text/value/reason/destination bytes; they are not serialized size or a token estimate.

The fixed all-semantics, empty, error, retained/evicted/navigation, and 512-control synthetic cases provide correctness and construction-size/time diagnostics. They do not constitute a task-efficiency or process-memory claim.

## Dependency boundary

`mawr-observation` is private at version `0.0.0`. Its only direct runtime dependencies are `mawr-core` and `mawr-state`. Repository verification rejects direct encoding, subprocess, or additional runtime dependencies so M10 transport decisions cannot leak into M5.
