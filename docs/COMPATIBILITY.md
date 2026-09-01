# Compatibility reporting policy

MAWR has typed core contracts and a tested native transport boundary, but no supported runtime compatibility profile yet. This file defines how broader compatibility will be reported; local fixture coverage is not a general web-support claim.

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

The native static engine has deterministic local evidence for the bounded HTTP(S), redirect, cookie, URL-encoded form, and download capabilities documented in [NATIVE-STATIC-ENGINE.md](NATIVE-STATIC-ENGINE.md). That evidence covers owned fixtures, not arbitrary public sites or a supported runtime profile. HTML parsing, semantic extraction, actions, JavaScript, layout, rendering, protocol encoding, and end-to-end task compatibility remain unsupported or unimplemented as stated by the engine capability report; they must not be inferred from transport success.

## Reporting rules

- Scope every claim to measured tasks, fixtures, and versions.
- Publish the capability matrix and failing cases, not only successful examples.
- Separate parsing, semantic extraction, action, protocol, and end-to-end task results.
- Distinguish unsupported from broken and untested.
- Never imply Chrome, Playwright, or general-browser compatibility from a compatibility protocol or external adapter.
- Link performance and efficiency claims to reproducible [benchmark artifacts](BENCHMARKS.md).

Authenticated live sites are not a reproducible compatibility corpus unless access, privacy, terms, reset behavior, and sanitization are explicitly resolved. Deterministic public fixtures are the default evidence base.
