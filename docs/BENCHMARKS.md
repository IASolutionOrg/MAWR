# Benchmark methodology

This is MAWR's canonical, normative benchmark methodology. Benchmark layout and contribution mechanics belong in [../benchmarks/README.md](../benchmarks/README.md). No benchmark harness or executable task exists during Phase 0; all commands, schemas, and artifact layouts described here are future contracts unless explicitly marked conceptual.

## Purpose and decision rule

The primary MVP question is whether an agent can complete the same supported web tasks with substantially less total model context and fewer model decisions than a defined reference baseline. Correctness gates efficiency: a reduction in tokens, calls, latency, or resources that materially harms task success is a regression.

The primary efficiency metric is **total provider-reported model tokens per successfully completed task**. Observation payload tokens are a secondary diagnostic. The complete [scorecard](SCORECARD.md) is always reported so improvements cannot hide failure, retries, latency, or resource transfer.

## Success

Every task has an independent machine-verifiable success check, such as fixture server state, returned structured data, a file hash, or a known state transition. Model output claiming completion is never proof. Failed attempts remain in raw and aggregate results.

## Baseline registry

Every baseline has a stable name and version plus documented:

- engine and exact version;
- observation and action formats;
- complete tool schema;
- system/driver instructions;
- capability limits;
- model and harness compatibility.

Results never compare against an undefined "normal browser."

### Reference Full-State Baseline

The required initial baseline is **Reference Full-State Baseline**, versioned independently from MAWR. It uses:

- the same native static engine;
- the same task and fixture revision;
- the same fixture reset and success checker;
- the same model harness and model configuration;
- equivalent action semantics;
- the full semantic representation at every decision;
- no relevance filtering;
- no observation token budget;
- no incremental observation diff.

This isolates MAWR's observation strategy from browser-engine compatibility. It is not Chrome, Playwright, or a general-browser baseline and must not be marketed as one.

Optional external baseline adapters may be added later. They are named, versioned, limitation-documented, and reported separately; their results cannot be merged into the Reference Full-State Baseline series.

## Ablations

The harness supports controlled feature ablations using the same canonical task, fixture, model, success checker, and limits. Initial variants are:

- full MAWR;
- no incremental diffs;
- no relevance ranking;
- no token budgeting;
- no compact encoding;
- no action batching.

Each variant changes only the named feature where possible. Metadata records unavoidable differences. Ablations attribute measured effects to individual components rather than to the entire runtime.

## Same-task harness controls

A task defines one canonical user-level instruction. Baseline, MAWR, and ablation modes receive that exact instruction; it is not rewritten to favor a driver. Interface-required system instructions and tool definitions may differ, but their full content or content-addressed artifact is retained and included in token accounting.

The common harness fixes or records:

- task and fixture revision;
- initial server, page, session, cookie, and storage state;
- model identifier and exact snapshot when available;
- reasoning effort, sampling/temperature, seed when supported, and maximum output;
- retry, timeout, request, token, and cost policies;
- caching policy;
- success criteria and checker version;
- allowed capabilities and network policy;
- driver, engine, protocol, encoder, tokenizer, and tool-schema versions.

Fixture state resets before every individual run. A reset failure invalidates that run before model execution and is recorded as an infrastructure exclusion.

## Paired and repeated trials

Publishable comparisons use paired repeated trials:

1. A pair uses the same task revision and model configuration.
2. Each member receives an independently reset fixture and session.
3. Baseline/MAWR execution order alternates or is randomized by a recorded policy.
4. Metadata records which mode ran first and any supported deterministic seed.
5. Repetition count is configured before execution.
6. All raw runs, failures, and exclusions are retained.

Reports show success rate, median, distribution or spread, completed-run count, failed-run count, excluded-run count, and each exclusion reason. They report paired differences where the data permits and do not claim statistical confidence unsupported by sample size or model variance.

Run order, fixture reset duration, warm/cold cache policy, and concurrency are part of the result. A comparison cannot silently mix cached and uncached modes or serial and contended resource conditions.

## Versioned tasks and fixtures

A public task schema is versioned before use. Its conceptual fields include:

```yaml
id: contact-form-basic
revision: 1
fixture: fixtures/contact-form@1
prompt: >
  Submit the contact form using the provided test identity.
initial_state: clean
inputs:
  identity: fixture-user-1
success:
  type: server_state
  expected:
    submission_created: true
limits:
  max_model_calls: 12
  max_retries: 2
allowed_capabilities: [forms, session_cookies]
```

The exact schema is not frozen in Phase 0. Every implemented task must define its canonical prompt, fixture and revision, initial state, test inputs, machine-verifiable success criteria, limits, and allowed capabilities.

The initial fixture plan covers:

1. article or documentation retrieval;
2. heavy navigation and boilerplate;
3. a large table;
4. a search flow;
5. a multi-field form;
6. a validation error and repair;
7. pagination;
8. thousands of irrelevant nodes;
9. page-state changes suitable for diffs;
10. an unsupported JavaScript dependency;
11. a download flow where feasible.

Fixtures are deterministic, local, versioned, independent of private services, and resettable. Their success checks fail closed and do not depend on model prose.

## Token accounting

The normalized usage model distinguishes:

- system and developer instructions;
- tool definitions;
- user task prompt;
- browser observations;
- other tool results;
- model output;
- cached and uncached input where reported;
- reasoning-token detail where reported.

Provider-reported usage is authoritative when available. The raw provider usage payload is stored beside a normalized representation with provider, model identifier, exact snapshot when available, and provider SDK/API version where practical.

When authoritative usage is unavailable, the value is labeled **ESTIMATED** with tokenizer and estimation method. An estimate is never presented as exact. Locally measured observation-payload tokens are recorded separately from end-to-end provider usage and are never substituted without classification.

Reasoning tokens may be a subset of provider-reported output tokens. The harness preserves the provider breakdown but never adds reasoning tokens to output a second time when already included. Cached input is similarly broken out without subtracting it from total input unless the provider's semantics and report label explicitly define that view.

Codex App or Codex CLI may orchestrate verification and inspect artifacts. That does not automatically expose Codex conversation usage. Exact model-backed comparisons require a provider API or another machine-readable authoritative source. A Codex-native run without such data is labeled estimated, observational, or unsupported for exact token comparison; visual usage indicators or inferred context size cannot support an exact before/after claim.

## Resource and operational measurement

The harness records wall latency, CPU time, peak resident memory per session, network bytes, model round-trips, retries, errors, and relevant limits. Measurement backend, platform, clock, sampling resolution, process-tree boundary, network accounting boundary, and known blind spots are metadata. Values that cannot be measured reliably are unavailable or estimated, never silently zero.

Instrumentation must not impose a substantial cost on normal runtime execution. Benchmark-only collectors activate only in Test Mode.

## Test Mode

MAWR will expose an explicit Test Mode. Conceptual forms such as `mawr test` or `mawr serve --test-mode` are design targets, not current commands and not frozen CLI syntax.

Test Mode supports baseline-only, MAWR-only, paired comparison, ablation, full-suite execution, structured traces, token accounting, resource measurements, and report generation. It is:

- disabled by default and explicitly activated;
- separated from production behavior;
- loopback/local-only by default;
- based primarily on deterministic local fixtures;
- protected against accidental private-network or external-site access;
- unable to log API keys, cookies, passwords, authorization headers, or secrets;
- capable of structured redaction before trace persistence;
- configured to write into ignored build/result directories.

Network access outside the fixture allowlist requires a distinct explicit opt-in and auditable policy. Redaction happens before durable output, and reports identify redaction without exposing the value.

## Model-backed tests

Model-backed end-to-end tests are required before MVP release but remain optional, explicit, and outside the default deterministic CI gate. They use deterministic fixtures, paired canonical tasks, and independent success checks. The provider and model are configurable; credentials come from environment variables or approved secret storage and are never committed or logged.

Absent credentials cause a clean documented skip. Execution requires explicit network and cost opt-in, with configured token, request, retry, time, and cost limits. Reaching a limit stops safely and records an incomplete/limited result rather than guessing success.

## Artifacts

The planned primary output is raw machine-readable data:

```text
target/mawr-bench/
  manifest.json
  runs/
    baseline-001.json
    mawr-001.json
  comparison.json
  report.md
```

Markdown reports are generated from raw data. Artifacts record task/fixture revision, driver, engine, protocol, model/provider and settings, prompt, system instructions, tool definitions or hashes, run order, reset/caching policy, success result, usage breakdown, model calls, retries, timing, CPU, memory, network, errors, exclusions, and exact/estimated classification.

Private prompts, secrets, authenticated site data, and unsanitized traces are never committed or published. Only deterministic public fixtures and sanitized results are eligible for publication.

## Reproducibility checklist

A public claim must provide or identify:

- source revision and clean/dirty status;
- platform and build profile;
- task, fixture, checker, harness, driver, engine, encoder, and tokenizer versions;
- model/provider configuration and usage source;
- complete limits, order, reset, cache, and concurrency policies;
- raw run artifacts and generated report;
- failures, exclusions, estimates, and known measurement gaps;
- enough instructions to rerun from a cross-platform terminal entrypoint.

Claims are scoped to measured tasks and configurations. They do not imply general web compatibility or performance.
