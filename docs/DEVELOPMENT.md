# Development and verification contract

MAWR is in pre-alpha implementation. The Rust workspace, repository verification entrypoint, M1 typed core contracts, M2 native HTTP(S) transport, M3 static HTML semantic extraction, M4 bounded local state identity, and M5 complete observation construction exist. Relevance selection, token budgeting, semantic diffing, protocol encoding, the benchmark harness, and the product CLI do not exist yet. This document distinguishes working repository commands from future contracts.

## Contributor workflow

1. Find the canonical contract in [README.md](README.md).
2. Keep changes scoped to a justified machine workload.
3. Update the contract, implementation, tests, and compatibility evidence together once code exists.
4. Run the smallest relevant deterministic checks, then the canonical verification entrypoint.
5. Inspect generated benchmark or compatibility artifacts for secrets before any publication.
6. Report exact commands, platform, failures, skips, and estimates.

## Toolchain and platform policy

The workspace uses Rust 2024 and has a minimum supported Rust version (MSRV) of 1.88.0. M2 raised the original language baseline because the first patched `time` release for RUSTSEC-2026-0009 requires Rust 1.88; retaining 1.85 would knowingly lock a vulnerable transitive dependency. Contributors normally use the current stable toolchain with the `rustfmt` and `clippy` components. Any further MSRV change requires an explicit technical decision supported by a language, standard-library, security, or dependency need.

Windows, Linux, and macOS are the intended host families. Stable Rust runs the canonical verification command on all three in CI; Rust 1.88.0 runs the same command on Linux as the dedicated MSRV gate. A revision is validated on a host only when that revision has a successful CI result for the host. This development matrix is not a runtime compatibility claim.

## Workspace and dependency policy

M0 began with the real cross-platform `xtask` package. M1 adds `mawr-core`, a private `0.0.0` package containing implemented domain contracts and tests. M2 adds `mawr-native-static`, a private `0.0.0` package containing the native transport and deterministic loopback fixtures. M3 adds private `mawr-semantic-html` for bounded static parsing, normalization, and public HTML fixtures. M4 adds private `mawr-state`, depending directly only on semantic and core contracts, for conservative identity matching and bounded retention. M5 adds private `mawr-observation`, depending directly only on state and core, for lossless full-state observations without selection or encoding. Product packages are added only when an accepted milestone gives them concrete behavior, tests, and an enforced dependency boundary. Empty or speculative crate skeletons are prohibited.

Every Rust dependency requires a concrete capability that the standard library and existing dependencies cannot reasonably provide, plus maintenance, security, MSRV, and license review. Features must be kept narrow, duplicate dependencies avoided, and default features disabled when they add unused capability. M2 uses Tokio, Reqwest, rustls, URL parsing, and bounded content-decoding support with a locked graph; unused Reqwest defaults such as system proxies, HTTP/2, and blocking APIs are disabled. Project-authored source is licensed under Apache-2.0 through the repository `LICENSE`; per-file license headers are not required.

Rust code follows standard formatting, linting, type safety, explicit errors, bounded resource use, and minimal unsafe code with documented invariants. Workspace lint policy currently forbids unsafe Rust. `mawr-core` has no normal, development, or build dependencies; `cargo xtask verify` enforces that complete dependency-graph boundary.

## Test layers

Implemented tests cover core construction and invariants; deterministic loopback HTTP, redirect, cookie, form, compression, resource-limit, cancellation, safe-download, destination-policy, and local TLS behavior; static semantic fixtures; state transitions, ambiguity, reset, eviction, stale lookup, memory bounds, and session isolation; and complete observation semantics, capabilities, bases, resets, bounds, omissions, and deterministic ordering. Later milestones will add terminal-runnable layers for:

- unit behavior;
- integration across owned components;
- deterministic fixture flows;
- agent protocol validation;
- shared engine-contract compliance;
- benchmark harness correctness;
- Reference Full-State Baseline versus MAWR;
- resource limits and failure behavior;
- security boundaries and session isolation;
- optional model-backed end-to-end agent tasks.

Default CI must be deterministic, local, credential-free, cost-free, and free of GUI requirements. Model-backed tests are a separately opted-in release gate governed by [BENCHMARKS.md](BENCHMARKS.md).

## Implemented verification entrypoint

The canonical repository command is:

```text
cargo xtask verify
```

It runs the following implemented checks with a locked dependency graph:

```text
cargo check --locked --workspace --all-targets
cargo tree --locked --package mawr-core --edges all --prefix none
cargo tree --locked --package mawr-native-static --edges all --prefix none --no-dedupe -e features
cargo tree --locked --package mawr-state --depth 1 --edges normal --prefix none
cargo tree --locked --package mawr-observation --depth 1 --edges normal --prefix none
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo xtask docs
```

The core dependency-graph check requires `mawr-core` to be the graph's only package. The native boundary check rejects browser/external-engine packages, Reqwest blocking/default/HTTP2/system-proxy features, and Rust process APIs in native-engine source. The semantic boundary verifies the selected parser stack and prohibits browser or subprocess fallback. The state boundary permits only direct `mawr-core` and `mawr-semantic-html` dependencies and prohibits subprocess use. The observation boundary permits only direct `mawr-core` and `mawr-state` dependencies and rejects encoding or subprocess APIs. `cargo xtask docs` validates local targets in Markdown links. The `verify`, `docs`, and `help` subcommands are the complete current `xtask` surface. Possible future subcommands include `smoke`, `benchmark`, `compare`, and `release-check`; they are not runnable commands until their milestone implements and verifies them.

The entrypoint returns zero only when every implemented check passes and non-zero on failure. It requires no GUI, credentials, external service, or IDE-specific runner; transport integration tests use owned loopback fixtures and a test-only local CA. Machine-readable result manifests, runtime smoke checks, benchmarks, and model-backed opt-in gates remain later milestone work and are not implied by the current command.

Codex App or CLI may invoke the entrypoint and summarize tests, benchmark comparisons, resource regressions, and release blockers. Codex is an orchestrator; exact model-token measurement still comes from an authoritative machine-readable provider source as defined in [BENCHMARKS.md](BENCHMARKS.md).

## Cross-platform requirements

Verification targets Windows, Linux, and macOS. Prefer Rust-native orchestration and portable paths over Bash-only scripts. Tests own their temporary resources, bind fixture services to loopback, avoid ambient credentials and user profiles, use deterministic time/randomness where needed, and cleanly distinguish unsupported, skipped, failed, and infrastructure-error outcomes.
