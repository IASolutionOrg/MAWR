# Core domain contracts

This document records the implemented M1 boundary in `mawr-core`. It complements the durable ownership in [ARCHITECTURE.md](ARCHITECTURE.md), [ENGINE-CONTRACT.md](ENGINE-CONTRACT.md), [PROTOCOL.md](PROTOCOL.md), [ENCODING.md](ENCODING.md), and [SECURITY-MODEL.md](SECURITY-MODEL.md); it does not replace those contracts or promise stable public Rust signatures.

## Implemented boundary

`mawr-core` is a private, dependency-free Rust crate containing MAWR-owned domain types shared by future engines, observations, actions, encoders, and harnesses. It currently defines:

- non-zero session IDs and session-scoped state, page, and element identities;
- structurally validated absolute URL values and page/engine identity;
- an exhaustive capability vocabulary with supported, unsupported, or constrained status;
- semantic roles, provenance, values, properties, relationships, and action affordances;
- typed observation requests and observations with explicit full, incremental, or reset basis;
- bounded observation budgets, unit collections, text, relationships, and capability constraints;
- typed single actions with expected-state and session validation;
- state transitions with explicit causes and reset reasons;
- structured operational failure classes and retry dispositions;
- exact, estimated, or unavailable measurement values.

Constructors validate cross-field invariants. References cannot cross sessions, capability reports belong to one engine, observation units are deterministically ordered and bounded, and sensitive action values and URL details have redacted debug output. Ambiguous absence is represented explicitly where it affects meaning, such as unavailable measurements, unknown semantic properties, unsupported capabilities, and observation resets.

## Dependency and encoding boundary

The crate has no normal, development, or build dependencies. The canonical verification command inspects its complete Cargo dependency graph and fails if another package appears. This keeps JSON, HTTP clients, parsers, CLI/MCP concerns, CDP, Chromium, and external-engine types outside the core model.

Serialization is intentionally absent. Compact JSON remains a future boundary owned by [ENCODING.md](ENCODING.md) and milestone M10. The crate version is `0.0.0`, `publish = false`, and its Rust API may evolve with later milestones until external protocol compatibility is deliberately specified.

## Deliberate limitations

M1 implements contracts, validation, and deterministic value behavior only. It does not navigate, resolve or authorize URLs, fetch or parse documents, extract semantics, retain state, rank relevance, encode messages, execute actions, expose a CLI or MCP server, or claim runtime/platform/web compatibility.

The current URL type verifies only bounded absolute structure. Full standards-compliant URL parsing, resolution, destination policy, HTTP behavior, and download handling belong to the native static engine milestone. Observation construction from live semantic state, batching, and incremental diff generation likewise remain later milestones.
