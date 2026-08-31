# Engine contract

MAWR owns an engine abstraction so the semantic, observation, and action layers do not depend on a particular runtime. This document fixes responsibilities and invariants, not exact Rust signatures.

## Conceptual surface

An engine conceptually provides operations equivalent to:

```rust,ignore
trait BrowserEngine {
    async fn navigate(/* MAWR request */) -> /* MAWR result */;
    async fn page_state(/* MAWR request */) -> /* MAWR state */;
    async fn execute(/* MAWR actions */) -> /* MAWR result */;
    fn capabilities(&self) -> EngineCapabilities;
}
```

The implementation milestone must choose precise ownership, async, error, and lifetime shapes. This sketch is not a frozen public Rust API.

## MAWR-owned inputs and outputs

Requests, responses, page state, actions, errors, and capabilities use MAWR domain types. CDP, Obscura, HTML-parser, HTTP-client, or other adapter types are translated at the adapter boundary. JSON is not the internal domain model.

Every state-producing result identifies the engine, capability set, navigation context, and state transition needed for reproducibility and stale-action protection.

## Capabilities

Capability reporting is explicit and versioned. At minimum it can distinguish support for:

- JavaScript;
- forms and individual control categories;
- downloads;
- layout and geometry;
- visual rendering;
- network observation;
- cookies and persistent storage.

A Boolean may be insufficient where support has limits; the eventual typed contract may carry levels, constraints, and reasons. Capability queries must not cause navigation or silently switch engines.

## Unsupported and failed operations

Unsupported behavior returns a structured `unsupported_capability`-class result naming the requested capability and current engine. Operational failures remain distinct, including navigation, protocol, parsing, authorization, timeout, resource-limit, stale-state, and adapter failures.

No failure may trigger an undeclared Chromium fallback or silently select a different engine. Callers may explicitly choose an allowed alternative after inspecting the result.

## Native static engine

The native Rust static engine is mandatory. Its initial intended capabilities cover HTTP and HTTPS, redirects, HTML parsing, semantic content and common form controls, form submission, session cookies, URL resolution, and downloads where practical. It must work with no external engine installed.

Initial explicit non-capabilities include JavaScript execution, CSS layout, visual rendering, screenshots, Canvas, WebGL, and media playback. Pages that require these features must produce truthful capability results.

## Optional dynamic engines

An external lightweight engine may eventually be supported for dynamic pages. It must be optional, replaceable, separately identifiable in compatibility and benchmark results, and isolated behind this contract. MAWR will not initially fork Obscura. External process startup, trust boundaries, resource use, and failures must be explicit.

## Contract verification

Each engine will share an engine-contract test suite for capability truthfulness, state production, action behavior, structured failures, session isolation, resource limits, and prohibited fallback detection. Compatibility reports identify the exact engine and version rather than attributing all adapter behavior to MAWR.
