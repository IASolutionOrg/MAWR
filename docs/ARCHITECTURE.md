# Architecture

This document owns MAWR's durable system boundaries. The dependency-free M1 domain types, M2 native HTTP(S) transport, M3 static HTML semantic extractor, M4 bounded local state store, M5 complete observation builder, M6 deterministic relevance/budget selector, M7 authorized static action executor, and M8 bounded batch executor exist; semantic diffs, encoding, and every later layer remain contracts for later MVP implementation rather than claims about shipped behavior. See [CORE-CONTRACTS.md](CORE-CONTRACTS.md), [NATIVE-STATIC-ENGINE.md](NATIVE-STATIC-ENGINE.md), [SEMANTIC-HTML.md](SEMANTIC-HTML.md), [STATE-STORE.md](STATE-STORE.md), [OBSERVATIONS.md](OBSERVATIONS.md), [RELEVANCE.md](RELEVANCE.md), [ACTIONS.md](ACTIONS.md), and [BATCHES.md](BATCHES.md) for the exact implemented boundaries.

## Core flow

```text
authorized web input
        |
        v
BrowserEngine -----> capability and structured failure
        |
        v
typed semantic state <---- bounded local state store
        |
        v
deterministic relevance ranking
        |
        v
complete-unit token selection
        |
        v
typed Observation -----> ObservationEncoder
        |                       |
        v                       v
      agent              compact JSON (MVP)
        |
        v
typed action batch + expected state
        |
        v
engine execution -----> new state -----> meaningful diff
```

## Layers and ownership

### Core contracts

`mawr-core` owns serialization-independent identity, capability, semantic, observation, action, transition, failure, and measurement values. It validates session and cross-field invariants without depending on an engine, parser, transport, or external protocol. Its internal Rust API remains intentionally evolvable while later milestones supply real producers and consumers.

### Engines

Engines acquire and mutate page state. The mandatory native static engine implements bounded HTTP(S) document acquisition, redirects, URL resolution, session cookies, URL-encoded GET/POST form transport, and explicit safe downloads. M3 turns its static `DocumentInput` into bounded semantic text and controls, while M7 executes the supported static mutations and returns through extraction/state. The boundary excludes JavaScript, layout, visual rendering, screenshots, Canvas, WebGL, and media playback.

Optional dynamic engines may extend capabilities but remain outside the core model. MAWR does not initially fork Obscura. No adapter may silently launch Chromium or leak vendor types into core contracts. [ENGINE-CONTRACT.md](ENGINE-CONTRACT.md) owns this boundary.

### Semantic model

The implemented static semantic model represents page, region, heading, text, link, form, textbox, checkbox, radio, select, option, button, table, row, cell, list, list item, and alert roles. It retains names, descriptions, values, state, relationships, HTTP(S) destinations, source-node provenance, and action affordances without making the raw DOM the normal agent interface. Its standards subset and CSS/script limitations are documented in [SEMANTIC-HTML.md](SEMANTIC-HTML.md).

M3 source-node IDs are deterministic only for one parsed document and are not action references. M4 maps them to compact `ElementRef` values that survive a defined conservative set of transitions when semantic identity is unique. References remain scoped to one session, are never recycled in that session, and do not become CSS selectors or globally durable identifiers.

### Local state and future diffs

`mawr-state` retains full semantic documents locally behind explicit state and page identities. Retention is bounded by both state count and total semantic units. Current-state reference lookup distinguishes stale states, missing references, and session mismatches; evicted history is never silently addressed. M5 exposes a typed change placeholder and reset basis, while semantic diff payloads remain an M9 concern. [STATE-STORE.md](STATE-STORE.md) owns the implemented lifecycle.

### Relevance and budgeting

The implemented M6 ranker is deterministic and local for fixed inputs. Its versioned configurable weights combine goal overlap in names, descriptions, and values; structural and interactive roles; caller-supplied changed-unit hints; alert and invalid-state priority; dependency context; and repeated-text/navigation penalties. Stable references break score ties.

Budgeting selects complete semantic units and their structural/label dependencies before encoding. The page, alerts, invalid controls, and their dependencies are essential: they may explicitly overshoot a tiny budget rather than disappear. Every other omission is counted as budget or irrelevance. Tokenizer identity/version and exact-versus-estimated quality are retained; the built-in UTF-8 heuristic is always estimated. No model is invoked. [RELEVANCE.md](RELEVANCE.md) owns this boundary.

### Observation and action boundary

The implemented M5 full observation contains state/page/engine identity, a bounded page summary, every semantic unit, capabilities, zero-omission metadata, construction measurements, and an explicit no-change/incremental-placeholder/reset basis. M6 derives a selected observation without mutating that reference input; semantic change payloads remain later work. M7 executes one typed navigate, follow, fill, select, check, uncheck, submit, or supported press after expected-state, reference, semantic, capability, validity, and caller-authorization preflight. M8 preflights bounded ordered batches on cloned state, preserves per-item authorization, and exposes exact partial runtime results; unknown post-navigation semantics terminate reference-bearing preflight. [OBSERVATIONS.md](OBSERVATIONS.md) owns the full builder, [RELEVANCE.md](RELEVANCE.md) owns selection, [ACTIONS.md](ACTIONS.md) owns single-action execution, [BATCHES.md](BATCHES.md) owns batch execution, and [PROTOCOL.md](PROTOCOL.md) owns the future external agent contract.

### Encoding boundary

Typed observations and actions are independent of transport. Compact JSON is the MVP encoding. Alternative encoders are future measured additions and cannot require engine, semantic, state, ranking, or action changes. [ENCODING.md](ENCODING.md) owns encoding rules.

## Future architectural directions

These directions are durable boundaries, not implementation sequencing:

- A network semantic layer may use authorized structured responses when it preserves origin, authorization, provenance, and action semantics and never bypasses access controls.
- Identity may add encrypted local profiles, cookies, storage, OAuth grants, and secret isolation; secret values stay out of model context unless strictly required and authorized.
- Official service connectors may use authorized APIs when they are safer or more efficient than UI automation.
- Workflow recall may record a verified procedure, replay it with state assertions, repair locally, and escalate to a model only when reasoning is required.
- Observation may escalate from semantics to geometry, targeted regions, and only then full visual data when the workload justifies it.
- Native capabilities may later add events, JavaScript, fetch/XHR, dynamic DOM, storage, selective layout, and selective paint only when benchmarks justify each step.
- Scale may add isolated session pooling, scheduling, resource budgets, suspension, serialization, and safe cache sharing.

Security authorization applies across every layer; see [SECURITY-MODEL.md](SECURITY-MODEL.md).
