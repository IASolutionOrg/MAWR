# Bounded semantic diffs: implemented M9 boundary

`mawr-observation` can compare the current retained state with an exact retained same-page base. The result is a deterministic typed delta over MAWR semantic units, not a raw DOM mutation stream, script event log, serialized patch, or cross-session history.

## Delta shape

`SemanticChanges` identifies the base and target states and carries sorted, duplicate-free, disjoint reference sets for additions, updates, and removals. Added and updated references have complete target `SemanticUnit` replacements in the observation. Removed references have no unit payload. A changed page summary carries the target summary, and a changed document order carries the complete target reference order. Unchanged units, summary, and order are omitted.

Stable references provide continuity. When semantic identity cannot be retained safely, the state transition records a reset rather than inventing continuity. Role changes that replace an identity therefore appear as removal plus addition; value, validation, alert, affordance, relationship, destination, parent, and other changes on a retained identity appear as full unit updates.

Identical states produce a computed delta with no emitted units or change flags. Canonical unit storage remains sorted by `ElementRef`; document order is a separate explicit sequence so reordering is observable without changing lookup determinism.

## Reconstruction and validation

`SemanticSnapshot::from_full` accepts a complete or reset observation and validates that its semantic order names every unit exactly once. `SemanticSnapshot::apply` accepts only a computed incremental observation whose basis, change base, target, session, and page match the snapshot. It validates the exact emitted reference set, applies removals and complete replacements, then validates the target order. Applying an eligible delta reproduces the target summary, units, and order exactly.

The relevance selector deliberately rejects incremental observations. Selection requires a complete semantic view; callers reconstruct locally or request/reset to full state before selecting.

## Bounds and reset behavior

Observation and change limits are independently configurable and default to 250,000 entries. Diff construction first bounds both retained inputs. Its entry budget counts unit additions, updates, removals, a changed summary, and every reference in a changed target order. If the input or result exceeds the change limit, the builder returns the complete target with `Reset(DiffTooLarge)`.

Evicted and unavailable bases return `BaseEvicted` or `BaseUnavailable`. Intervening state resets preserve their exact reason, including navigation and ambiguous identity. Bases from another session are invalid input. No case silently treats an unsafe or missing base as an eligible delta.

## Diagnostics and evidence

Build diagnostics retain full-versus-emitted unit counts and logical-content bytes. Diff diagnostics add base/target/emitted unit counts, added/updated/removed counts, summary/order flags, bounded entry count, and construction latency. These are local typed-content measurements, not encoded bytes, provider tokens, peak memory, or a task-efficiency claim.

Tests cover all semantic properties, reorder, add/remove, role-reference replacement, relationships, navigation, form validation through a real local action, alerts, eviction, ambiguous reset, identical states, oversized fallback, deterministic generated transitions, and exact reconstruction. The common benchmark harness and no-diff task ablation remain M13 work.

## Dependency boundary

M9 adds no runtime dependency. `mawr-observation` remains private at version `0.0.0` with only `mawr-core` and `mawr-state` as direct runtime dependencies. Encoding is intentionally absent and remains owned by M10.
