# Authorized static actions: implemented M7 boundary

M7 adds the private `mawr-actions` crate. It executes one typed `ActionRequest` against the current `mawr-state` state through the existing native transport and semantic extractor. This remains a Rust library boundary: there is no public wire format, CLI command, MCP tool, batching layer, JavaScript engine, visual click, or upload implementation.

## Preflight and authority

Construction requires a caller-supplied `ActionAuthorizer`; there is no implicit mutation authority. Before any local state update or network request, the executor verifies the expected current state and session, every element reference, native HTML interaction kind, disabled/control state, advertised affordance, required capability, supported form method and encoding, constraint state, destination, and authorization decision. A rejected preflight reports `SideEffectStatus::NotStarted`.

Authorization receives only typed audit context: requested/effective action kind, operation, expected state, target, destination, and request method. Fill values and form values are deliberately absent. Semantic interaction and hidden-control debug output reports presence/count metadata but redacts web values.

## Supported semantics

- `navigate` performs an authorized native GET and re-enters semantic extraction and state acquisition.
- `follow` accepts native HTTP(S) links. A link carrying `download` is rejected before network access because `Follow` has no explicit download root or safe filename authority.
- `fill` updates a supported native text control locally; password values remain semantically and diagnostically redacted.
- `check` and `uncheck` update native checkboxes. Checking a radio clears the same form/name group.
- `select` validates that the option belongs to the target native select and applies single- or multiple-select state.
- `submit` supports native forms and submitters using URL-encoded GET or POST. Successful controls preserve document order and duplicate names; hidden values, checked controls, selected options, and only the chosen submitter are included. Disabled and unnamed controls are omitted. GET ignores `enctype` as HTML requires, while `novalidate` and `formnovalidate` bypass static constraint checks.
- `press` maps only deterministic static behavior: Enter on a link, Space on checkbox/radio, Enter or Space on a submitter, and Enter on a text control with a form. Generic button behavior is explicitly unsupported as JavaScript; other keys are unsupported key input.

Multipart or `text/plain` POST, unknown form encodings/methods, file uploads, image-submit coordinates, JavaScript events, and visual interaction fail explicitly instead of being approximated.

## Results and state

Every success returns requested and effective action kinds, the exact `StateUpdate`, optional network evidence, and measured wall-clock action latency. Network evidence includes method, requested/final URL, HTTP status, request/redirect counts, and decoded body bytes. If HTTP completes but parsing or state acquisition fails, the error retains that evidence with `SideEffectStatus::NetworkCompleted`; the previous semantic state remains current. A transport attempt that fails reports `Requested` without pretending that the server did or did not observe it.

Local mutations are applied to a cloned semantic document and enter the transactional state store only after all preflight checks and authorization succeed. Stable references are then reassigned by the existing M4 rules.

## Verification boundary

Owned loopback tests cover all eight action kinds, local control transitions, radio groups, select ownership, URL-encoded GET and POST, duplicate field names, hidden values, chosen submitters, required validation, disabled controls, press mappings, stale/missing/cross-session references, denied authorization, download intent, navigation transitions, and completed-network parse failures. The repository verifier enforces that `mawr-actions` has direct runtime dependencies only on core, native transport, semantic HTML, and state, with no encoding, browser, or subprocess fallback.
