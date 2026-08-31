# Compatibility reporting policy

MAWR has no runtime implementation or compatibility profile yet. This file defines how compatibility will be reported once executable capability evidence exists; it makes no claim of current support.

## Capability reporting

Compatibility is reported as explicit, versioned capabilities and limits rather than a single browser-like percentage. Reports identify:

- MAWR revision and release;
- engine and exact version;
- platform and build profile;
- protocol and encoding version;
- fixture or site revision and access conditions;
- supported, partially supported, unsupported, and untested capabilities;
- structured failure observed;
- reproducible test evidence.

Engine capability truthfulness is part of the contract suite. A native-static result is not merged with an optional dynamic-engine result, and an adapter failure cannot be presented as native support.

## Current status

No engine, platform, protocol, encoding, or web capability has been implemented or validated. The native static engine and its initial capability boundary are design targets described in [ENGINE-CONTRACT.md](ENGINE-CONTRACT.md), not a current compatibility matrix. Unsupported and untested capabilities must not be inferred from planned scope.

## Reporting rules

- Scope every claim to measured tasks, fixtures, and versions.
- Publish the capability matrix and failing cases, not only successful examples.
- Separate parsing, semantic extraction, action, protocol, and end-to-end task results.
- Distinguish unsupported from broken and untested.
- Never imply Chrome, Playwright, or general-browser compatibility from a compatibility protocol or external adapter.
- Link performance and efficiency claims to reproducible [benchmark artifacts](BENCHMARKS.md).

Authenticated live sites are not a reproducible compatibility corpus unless access, privacy, terms, reset behavior, and sanitization are explicitly resolved. Deterministic public fixtures are the default evidence base.
