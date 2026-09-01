# MAWR

**Machine-Aware Web Runtime**

MAWR is an open-source, Chromium-free web runtime designed for machines and AI agents. Traditional browsers turn the web into pixels for humans; MAWR aims to turn the web into actionable state for machines.

## Status

MAWR is **pre-alpha and in its implementation phase**. Phase 0 established the project contracts, M0 established the verified Rust workspace, M1 implements the dependency-free typed core contracts, M2 implements bounded native HTTP(S) transport, and M3 implements bounded deterministic [static HTML semantic extraction](docs/SEMANTIC-HTML.md). These are Rust library boundaries, not yet a usable browser product: stable cross-state references, action execution, ranking, encoding, CLI, MCP server, installable packaging, and a supported compatibility profile do not exist yet.

The implemented repository checks are documented in [the development guide](docs/DEVELOPMENT.md). Runtime commands and installation instructions will be documented only after working implementations exist. Product documents describe intended contracts, not shipped runtime behavior or measured performance.

## Direction

MAWR is intended to expose compact semantic state, select complete relevant units under a token budget, accept deterministic action batches, and return meaningful state changes. Correct task completion is the first constraint; efficiency is measured as total model tokens per successfully completed task alongside latency, model round-trips, CPU, memory, network use, and retries.

The project has three permanent constraints:

- Rust is the primary implementation language.
- Chromium, Blink, Electron, and hidden Chromium fallbacks are forbidden.
- A native static engine must work independently of any optional, replaceable dynamic engine.

MAWR is not a general-purpose browser for humans. Features that only improve human presentation do not belong unless machine workloads and reproducible measurements justify them.

## Documentation

Start with the [documentation index](docs/README.md). The canonical public documents cover:

- [vision](docs/VISION.md) and [principles](docs/PRINCIPLES.md);
- [architecture](docs/ARCHITECTURE.md), [implemented core contracts](docs/CORE-CONTRACTS.md), [native static engine](docs/NATIVE-STATIC-ENGINE.md), [static HTML semantics](docs/SEMANTIC-HTML.md), [engine contract](docs/ENGINE-CONTRACT.md), and [agent protocol](docs/PROTOCOL.md);
- [encoding](docs/ENCODING.md), [benchmark methodology](docs/BENCHMARKS.md), and the [scorecard](docs/SCORECARD.md);
- [security model](docs/SECURITY-MODEL.md), [compatibility policy](docs/COMPATIBILITY.md), and [development contract](docs/DEVELOPMENT.md).

See [CONTRIBUTING.md](CONTRIBUTING.md) before proposing changes. Security reports follow [SECURITY.md](SECURITY.md).

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
