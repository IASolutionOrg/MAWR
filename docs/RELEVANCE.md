# Deterministic relevance and token budgeting: implemented M6 boundary

`mawr-relevance` converts an immutable complete M5 observation into a typed selected observation without a model call or transport encoding. Inputs with prior omissions or without exactly one page unit fail closed. Fixed observation, request, ranking configuration, tokenizer, and change hints produce the same selected units, scores, omissions, and diagnostic trace. Runtime latency is the only intentionally variable diagnostic.

## Ranking signals

The versioned `RankingConfig` owns configurable signed weights and the minimum eligible score. The default `mawr-relevance-v1` profile combines:

- normalized Unicode-alphanumeric goal overlap in known names, descriptions, text values, and explicit unknown-value reasons;
- interactive and structural-role bonuses;
- essential alert and invalid-state priority;
- caller-supplied changed-reference hints, validated against the observation session and unit set;
- parent and label/description/ownership relationship context;
- penalties for text repeated at least three times and links that repeat the same normalized name and destination.

Scores use saturating arithmetic. Descending score establishes candidate order and stable `ElementRef` is the final tie-breaker. The external diagnostic trace contains only references, numeric scores/costs, booleans, and overlap counts; it does not copy goal or page text.

## Complete-unit and context packing

Units are projected and counted independently, then packed before any M10 encoding exists. A selected candidate is atomic and brings its transitive semantic parent plus labelled-by, described-by, owned-by, option, row, cell, or list ownership targets. A bundle either fits or remains omitted; text or bytes inside a unit are never truncated.

The page, alerts, controls with known invalid state, and their context dependencies are essential. They are included even when the requested budget is too small; the exact overshoot is recorded rather than hiding critical state. Non-essential candidates never overshoot. Without `max_tokens`, M6 preserves every M5 unit and produces token diagnostics without filtering.

The request budget covers the typed observation projection plus an explicit configurable reserve for later envelopes/tool framing. The default reserve is zero, and there is no hidden slack. If reserve and envelope consume the entire budget, the effective unit budget is zero. Omitted units are counted as `Budget` when relevant/contextual but unable to fit, or `Irrelevant` when below the configured threshold.

## Tokenizer contract

`TokenCounter` is a provider-neutral interface over independently framed diagnostic fragments. Metadata always records tokenizer name, version, and `Exact` or `Estimated` quality. An implementation may claim exactness only for its declared additive fragment algorithm and must count every non-empty fragment as at least one token; zero fails closed. MAWR's built-in `utf8-bytes-div-4@1` fallback counts `ceil(UTF-8 bytes / 4)` and is always estimated.

M6 does not silently choose a provider/model tokenizer. Provider-specific exact tokenizers remain an explicit caller integration and future M14 conformance concern. M10 must separately measure its final serialized payload as a whole; M6 projection tokens and bytes cannot be relabelled as wire or provider usage.

## Diagnostics and boundary

The selected observation records its local observation-token measurement while preserving other M5 measurements. Separate diagnostics expose ranking/tokenizer versions, requested/reserved/envelope/projected tokens, essential overshoot, input/selected/omitted unit counts, projection bytes, selection latency, and the content-free score trace.

Public fixed tests cover isolated signals, stable ties, Unicode tokenizer variance, zero effective and tiny budgets, oversized essentials, form context, alerts, validation, changed hints, repeated navigation, 1,000 irrelevant nodes, custom weights, invalid references, deterministic repeats, and budget/subset properties. These diagnostics are not a task-success, wire-size, model-token, memory, or cross-platform performance claim.

`mawr-relevance` is private at version `0.0.0`; its only runtime dependency is `mawr-core`. Repository verification rejects model, HTTP, serialization, and subprocess APIs in its source.
