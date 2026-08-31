# Architectural principles

## Machine value over browser resemblance

> If a feature makes MAWR more browser-like but does not make machines better at using the web, it probably does not belong in MAWR.

Features must be justified by supported machine workloads and measured end-to-end outcomes, not browser feature parity.

## Correctness before compression

Task success is a gate for every efficiency claim. MAWR measures total task cost, includes failed runs, and never treats a model's claim of success as proof.

## No Chromium

MAWR must not bundle or launch Chromium or Chrome, use Blink as its runtime, depend on Electron, or hide Playwright Chromium behind an abstraction. Compatibility protocols such as CDP do not grant permission to use Chromium.

## Native static path

A native Rust static engine is mandatory and must operate without an external dynamic engine. Optional engines must be explicit, replaceable, capability-described, and isolated behind MAWR-owned contracts. Failure cannot silently change engines.

## Semantic state, not pixels or raw DOM by default

The primary page representation is typed, machine-oriented semantic state. Visual or geometric escalation is selective and justified by task need.

## Deterministic local selection

Initial relevance ranking and token-budget selection are local and deterministic for the same state, goal, configuration, and tokenizer. MAWR does not call another model merely to decide what page content to show a model.

## Complete units under a budget

Selection happens over meaningful semantic units before encoding. Serializing everything and truncating bytes or tokens is not a valid budgeting strategy.

## Typed core, replaceable edges

Core domain types belong to MAWR. JSON, compact text, TOON, CDP, and engine-specific formats are boundary representations, not internal models.

## Explicit capability and failure

Unsupported behavior is a structured result. Capabilities, state versions, authorization boundaries, and benchmark measurement quality must be visible rather than inferred.

## Reproducibility over marketing

Compatibility and performance claims require versioned fixtures, defined baselines, raw results, machine-verifiable success, and documented limitations.

## Minimal and cross-platform

Dependencies, model-facing tools, retained state, network traffic, and instrumentation should be minimized. Verification must run from a terminal on Windows, Linux, and macOS without a GUI or IDE-specific runner.
