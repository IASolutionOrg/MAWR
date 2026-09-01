# Security model

This document owns MAWR's technical threat model and security boundaries. Vulnerability disclosure procedure belongs only in [../SECURITY.md](../SECURITY.md). M2 implements the native transport subset described in [NATIVE-STATIC-ENGINE.md](NATIVE-STATIC-ENGINE.md), and M3 implements bounded static parsing described in [SEMANTIC-HTML.md](SEMANTIC-HTML.md); actions, persistence, external engines, and release remain unimplemented.

## Assets and trust zones

Protected assets include user intent, credentials, cookies, storage, OAuth grants, downloaded data, local files, model context, action authority, benchmark traces, and session state.

Trust zones include the user/agent boundary, MAWR core, engine adapters and external processes, network destinations, web content, model/provider APIs, fixture servers, and persisted artifacts. Each crossing requires explicit typed data, validation, least authority, and auditable failure.

## Web content is untrusted

Page text, metadata, scripts, network responses, forms, and downloaded content are data, not user or developer instructions. Semantic extraction must preserve provenance so an agent or policy layer can distinguish page content from trusted task instructions. Prompt injection is not considered solved by semantic representation, filtering, or model prompting.

## Network boundary and SSRF

Navigation, redirects, subresources, downloads, and structured network access can reach loopback, link-local, private, cloud-metadata, and internal services. URL parsing, DNS resolution and rebinding, redirects, proxy behavior, IPv4/IPv6 forms, and destination changes require one coherent policy. Denials are explicit. Authorized API or network-semantic access preserves origin, user authorization, provenance, and action semantics and never bypasses access control.

The implemented M2 transport applies its destination policy to every redirect hop, requires every resolved address to be authorized, pins approved addresses into the client, verifies the connected peer, disables ambient proxies, and rejects URL credentials. Loopback or arbitrary explicit destinations and ports require construction-time opt-in.

Test Mode is loopback fixture-only by default but must prevent a hostile fixture from pivoting to other private destinations. Any external or private-network access is a separate explicit opt-in.

## Actions and authorization

Observation authority does not imply mutation authority. Actions are typed and checked against session, domain, capability, and expected state before side effects. Higher-impact actions may require human confirmation. Batches cannot bypass individual checks and must report partial execution accurately.

## Secrets and model context

Future profiles and provider integrations isolate secrets from page state, logs, model observations, errors, and benchmark artifacts. Credentials are obtained from approved secret storage or environment at the boundary and are revealed to a destination only when authorized. The model receives a secret value only when unavoidable, scoped, and explicitly permitted.

Cookies, authorization headers, passwords, OAuth grants, API keys, storage values, and sensitive form values are redacted before trace persistence. Redaction must cover structured fields and error paths; it cannot rely only on string matching.

## Engine trust boundary

The native engine and every optional external engine run with documented process, filesystem, network, and resource authority. Adapter types are untrusted at the core boundary and translated into validated MAWR types. External-engine crashes, protocol confusion, capability mismatch, and startup failures are explicit. Silent fallback, especially to Chromium, is forbidden.

## Session isolation and state

Sessions isolate cookies, storage, references, caches, downloads, authorization, and retained semantic state. Stable references are not reusable across sessions. Bounded retention, resource quotas, safe cleanup, and serialization formats must prevent cross-session data leakage. Cache sharing is disabled unless content identity, authorization, and privacy invariants are proven.

## Resource and parser safety

Untrusted HTML, headers, compression, redirects, tables, forms, downloads, and state changes can cause excessive CPU, memory, disk, network, recursion, or token output. Implementations enforce bounded parsing, decompression, redirect, response, download, state, observation, and action limits with structured errors.

## Test and benchmark artifacts

Test Mode is explicit, disabled by default, separated from production paths, and writes to ignored result directories. Structured traces can contain sensitive content and are redacted before disk write. Public artifacts are limited to deterministic fixtures and sanitized results. Model-backed tests require explicit network/cost opt-in and never log provider credentials.

## Audit and unresolved limitations

Security-relevant decisions should produce a local audit trail without secret values: requested capability, authorization decision, destination, action category, state ID, result, and redaction status. Audit retention and access are bounded.

MAWR does not claim to solve prompt injection, hostile browser content, model misbehavior, or confused-deputy risks in general. Releases must state supported boundaries and residual risk truthfully.
