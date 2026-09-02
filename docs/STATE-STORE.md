# Local state and stable references: implemented M4 boundary

`mawr-state` converts each bounded `SemanticDocument` into a session-owned `StoredState`. It assigns compact `ElementRef` values, translates relationships whose targets are semantic units, retains a bounded state window, and rejects stale or cross-session lookup. It does not execute actions, build observations, rank content, emit diffs, or define a public wire format.

## Identity policy

References are monotonically allocated and never reused within a `SessionId`. Matching never uses M3 `SourceNodeId` values across documents because those values describe one parse only. For the same exact final document URL, matching proceeds in this order:

1. the single page unit retains its reference;
2. a unique bounded HTML `id` matches the same role with the same unique `id`;
3. an element without an author identity may match a unique fingerprint of semantic role, known accessible name, and known HTTP(S) destination;
4. every unmatched element receives a new reference.

Mutable values and control states are deliberately excluded from identity. Author IDs marked ambiguous or unsupported never fall back to a weaker semantic match. A semantic fingerprint must be one-to-one in both states; duplicates, collisions, unknown identity, or a role mismatch fail closed by allocating new references. Diagnostics distinguish unique author/semantic matches, new or identity-less elements, ambiguous author IDs, and ambiguous semantic fingerprints.

This is a conservative safety policy, not browser node identity. A reference may be lost when an un-IDed element is renamed, when duplicates appear, or when available semantic evidence changes. Losing a reference is preferred to addressing the wrong element.

## Reset and lifecycle policy

An exact final URL change, an explicit `Navigation` cause, or an explicit reset creates a new `PageId` and invalidates every prior element reference. The transition records navigation or the requested `ResetReason`; diagnostics enumerate the lost references and reset reason. Same-URL refreshes and external changes remain eligible for matching.

Every update receives a monotonically increasing `StateId`. Failed validation is transactional: it does not publish a state or consume state, page, or element sequences. Retained states are limited by both `retained_states` and aggregate `retained_units`; inserting a state evicts the oldest history until both limits hold. A single document larger than the unit-retention limit fails explicitly instead of evicting or truncating itself.

Retained history supports later observation and diff construction. Only the current state may resolve a reference for future action execution. Lookup returns structured outcomes:

- a session mismatch is invalid input;
- an evicted or non-current expected state is `StaleState` with the current state when available;
- an absent reference in the current state is `MissingReference`.

## Relationships and diagnostics

Relationships targeting another emitted semantic unit are translated from source provenance into stable references. Targets such as non-semantic label nodes remain explicitly available as unresolved extracted relationships; the store does not invent a semantic target. Immediate parents are translated only when that parent itself produced a semantic unit.

Each update reports preserved, newly allocated, and lost reference counts; assignment and loss reasons; evicted state IDs; retained state/unit counts; reset reason; and exact wall-clock matching latency from a runtime counter. The synthetic transition fixture exercises 512 reordered controls and exposes survival and retention diagnostics. It is correctness evidence, not a publishable performance or memory claim: exact process memory measurement and the common benchmark executable remain later milestones.

## Resource and implementation boundary

The default window retains at most 16 states and 500,000 semantic units, with hard configuration caps of 1,024 states and 10,000,000 units. Matching uses ordered maps and one-to-one buckets rather than pairwise element comparison, keeping the implemented path bounded by `O(n log n)` time and `O(n)` temporary identity data for `n` semantic units.

`mawr-state` is private at version `0.0.0`. Its only direct dependencies are `mawr-core` and `mawr-semantic-html`; the repository verification entrypoint enforces that boundary and prohibits subprocess fallback.
