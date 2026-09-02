# Agent protocol

This document owns the conceptual agent-facing observation and action contract. Exact wire schemas and names remain to be versioned during implementation.

No public agent protocol, MCP tools, or wire implementation exists yet. M5 implements the internal typed complete-observation producer described in [OBSERVATIONS.md](OBSERVATIONS.md), and M6 implements its deterministic [relevance and budget selector](RELEVANCE.md); the concepts below establish future transport ownership and invariants without freezing exact schemas prematurely.

## Small tool surface

The preferred primary interface exposes very few tools, conceptually `browser_observe` and `browser_act`. Tool definitions themselves consume model input tokens, so additional tools require measured task value.

## Observe

An observation request may include:

- a task goal used only for deterministic local relevance;
- `max_tokens`, identifying the observation budget and tokenizer policy;
- `since_state`, requesting changes relative to retained state.

An observation returns complete semantic units and includes:

- a new or current state ID;
- page identity and concise summary;
- selected semantic elements with compact references such as `e1`;
- meaningful changes since the requested state when available;
- an omission summary explaining hidden categories or counts;
- measurement and capability metadata.

The full state stays local. The protocol does not normally expose raw DOM, giant selectors, screenshots, or a byte-truncated serialization.

## Semantic roles and relationships

The protocol can represent page, region, heading, text, link, form, textbox, checkbox, radio, select, option, button, table, row, cell, list, list item, and alert. Elements carry only task-useful labels, values, states, relationships, and action affordances. Stable references are session-scoped handles, not durable website identifiers.

## Act

Typed actions include:

- navigate to an authorized URL;
- follow or click a semantic target;
- fill a field;
- select an option;
- check or uncheck a control;
- submit a form;
- press a supported key or semantic command.

An action request names the expected state. A stale expected state prevents execution and returns a structured result that lets the caller observe again. Actions include capability and authorization checks before side effects.

## Batches

Independent deterministic operations may be grouped to reduce model round-trips. Batch semantics must define ordering, validation, stop/continue policy, partial results, and resulting state. A batch cannot weaken per-action authorization or conceal side effects. Implementations should favor a fail-before-side-effect policy when preconditions can be checked upfront.

## Results and state transitions

Results distinguish successful execution, unsupported capability, invalid input, missing or stale reference, authorization denial, resource limit, navigation or engine failure, and partial batch completion. Success identifies the resulting state and meaningful changes. Model prose is never the authoritative success check for benchmark tasks.

## Versioning

Wire formats, tool schemas, and compatibility versions will be explicit. Additive and breaking changes must be reported separately, and benchmark artifacts must retain the tool definition or a content hash so token costs and behavior can be reproduced.
