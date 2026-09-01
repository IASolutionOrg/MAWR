# Architecture

This document owns MAWR's durable system boundaries. The dependency-free M1 domain types exist, but no runtime behavior currently exists; the flow and layers below remain contracts for later MVP implementation rather than claims about shipped behavior. See [CORE-CONTRACTS.md](CORE-CONTRACTS.md) for the exact implemented boundary.

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

Engines acquire and mutate page state. The mandatory native static engine targets HTTP(S), redirects, HTML parsing, semantic text and controls, forms, cookies, URL resolution, and practical downloads. Its initial boundary excludes JavaScript, layout, visual rendering, screenshots, Canvas, WebGL, and media playback.

Optional dynamic engines may extend capabilities but remain outside the core model. MAWR does not initially fork Obscura. No adapter may silently launch Chromium or leak vendor types into core contracts. [ENGINE-CONTRACT.md](ENGINE-CONTRACT.md) owns this boundary.

### Semantic model

The semantic model represents page, region, heading, text, link, form, textbox, checkbox, radio, select, option, button, table, row, cell, list, list item, and alert roles. It retains relationships and action affordances without making the raw DOM the normal agent interface.

Elements receive compact stable references. A reference should survive a reasonable state transition when the underlying semantic identity remains, but it is scoped to a session and state history; it is not a giant CSS selector or a globally durable identifier.

### Local state and diffs

Full semantic state remains local. States have explicit IDs and bounded retention. A subsequent observation can name a prior state and receive meaningful additions, removals, changes, alerts, and action results. If the requested base is unavailable, MAWR returns an explicit reset/full-state condition rather than fabricating a diff.

### Relevance and budgeting

Ranking is deterministic and local for fixed inputs. Candidate signals include goal, accessible name, label and text overlap; heading, form, and semantic proximity; interactive, changed-state, error, and alert bonuses; and boilerplate or repeated-navigation penalties.

Budgeting selects complete semantic units before encoding. It records what was omitted and whether local token measurements are exact for the selected tokenizer or estimated. Ranking never invokes a second model merely to filter a page.

### Observation and action boundary

An observation contains a state ID, concise page summary, selected elements, meaningful changes, omission summary, and metrics. Actions are typed operations such as navigate, follow, fill, select, check, uncheck, submit, and press. Deterministic actions may be batched, with expected-state validation preventing stale execution. [PROTOCOL.md](PROTOCOL.md) owns the agent contract.

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
