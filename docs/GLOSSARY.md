# Glossary

**Action** — A typed, authorized request to change navigation or page state, such as follow, fill, select, or submit.

**Action batch** — Ordered deterministic actions submitted together with expected-state validation to reduce model round-trips.

**BrowserEngine** — The conceptual MAWR-owned abstraction for navigation, state acquisition, action execution, and capability reporting; exact Rust signatures are not yet frozen.

**Capability** — An explicit, versioned statement of engine or runtime support and its limits.

**Compact JSON** — The first planned external encoding for typed MAWR observations and actions; it is not the internal data model.

**Engine** — A component that acquires and mutates page state. The native static engine is mandatory; optional engines remain replaceable.

**Expected state** — The state ID an action was prepared against, used to reject stale actions.

**Incremental diff** — Meaningful semantic changes relative to a retained prior state, not a raw DOM or byte diff.

**MAWR** — Machine-Aware Web Runtime, a Chromium-free runtime designed for machine use of the web.

**Observation** — A typed, budgeted view containing state identity, page summary, selected semantic units, changes, omissions, capabilities, and metrics.

**Reference Full-State Baseline** — The required baseline using the same native static engine and tasks but emitting full semantic state on every decision without relevance filtering, budgeting, or incremental diffs.

**Relevance ranking** — Deterministic local scoring of semantic units against goal, state change, affordances, structure, and boilerplate signals.

**Semantic role** — Machine-useful function of an element, such as heading, link, textbox, button, table, or alert.

**Semantic state** — MAWR-owned typed representation of page meaning, relationships, values, and actions; it is not normally raw DOM or pixels.

**Stable reference** — Compact session-scoped identity such as `e1` that survives reasonable semantic state transitions when possible.

**Test Mode** — Planned explicit, disabled-by-default execution mode for fixtures, baselines, ablations, traces, and measurements.

**Token budget** — Maximum observation budget applied by selecting complete semantic units before encoding, never by truncating a full serialization.

**TOON** — A possible future external encoding to be evaluated empirically; it is not an MVP dependency or current decision.
