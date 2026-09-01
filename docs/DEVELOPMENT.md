# Development and verification contract

MAWR is in pre-alpha implementation. The Rust workspace, repository verification entrypoint, and M1 typed core contracts exist. Runtime behavior, protocol encoding, the benchmark harness, and the product CLI do not exist yet. This document distinguishes working repository commands from future contracts.

## Contributor workflow

1. Find the canonical contract in [README.md](README.md).
2. Keep changes scoped to a justified machine workload.
3. Update the contract, implementation, tests, and compatibility evidence together once code exists.
4. Run the smallest relevant deterministic checks, then the canonical verification entrypoint.
5. Inspect generated benchmark or compatibility artifacts for secrets before any publication.
6. Report exact commands, platform, failures, skips, and estimates.

## Toolchain and platform policy

The workspace uses Rust 2024 and has an initial minimum supported Rust version (MSRV) of 1.85.0. Contributors normally use the current stable toolchain with the `rustfmt` and `clippy` components. Raising the MSRV requires an explicit technical decision supported by a language, standard-library, or dependency need.

Windows, Linux, and macOS are the intended host families. Stable Rust runs the canonical verification command on all three in CI; Rust 1.85.0 runs the same command on Linux as the dedicated MSRV gate. A revision is validated on a host only when that revision has a successful CI result for the host. This development matrix is not a runtime compatibility claim.

## Workspace and dependency policy

M0 began with only the real cross-platform `xtask` package. M1 adds `mawr-core`, a private `0.0.0` package containing implemented domain contracts and tests. Product packages are added only when an accepted milestone gives them concrete behavior, tests, and an enforced dependency boundary. Empty or speculative crate skeletons are prohibited.

Every Rust dependency requires a concrete capability that the standard library and existing dependencies cannot reasonably provide, plus maintenance, security, MSRV, and license review. Features must be kept narrow, duplicate dependencies avoided, and default features disabled when they add unused capability. The initial workspace intentionally has no third-party Rust dependencies. Project-authored source is licensed under Apache-2.0 through the repository `LICENSE`; per-file license headers are not required.

Rust code follows standard formatting, linting, type safety, explicit errors, bounded resource use, and minimal unsafe code with documented invariants. Workspace lint policy currently forbids unsafe Rust. `mawr-core` has no normal, development, or build dependencies; `cargo xtask verify` enforces that complete dependency-graph boundary.

## Test layers

Implemented core tests cover construction, validation, session scoping, exhaustive vocabularies, deterministic equality, bounded inputs, and the dependency boundary. Later milestones will add terminal-runnable layers for:

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
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo xtask docs
```

The dependency-graph check requires `mawr-core` to be the graph's only package. `cargo xtask docs` validates local targets in Markdown links. The `verify`, `docs`, and `help` subcommands are the complete current `xtask` surface. Possible future subcommands include `smoke`, `benchmark`, `compare`, and `release-check`; they are not runnable commands until their milestone implements and verifies them.

The entrypoint returns zero only when every implemented check passes and non-zero on failure. It requires no GUI, credentials, external service, or IDE-specific runner. Machine-readable result manifests, runtime smoke checks, benchmarks, and model-backed opt-in gates remain later milestone work and are not implied by the current command.

Codex App or CLI may invoke the entrypoint and summarize tests, benchmark comparisons, resource regressions, and release blockers. Codex is an orchestrator; exact model-token measurement still comes from an authoritative machine-readable provider source as defined in [BENCHMARKS.md](BENCHMARKS.md).

## Cross-platform requirements

Verification targets Windows, Linux, and macOS. Prefer Rust-native orchestration and portable paths over Bash-only scripts. Tests own their temporary resources, bind fixture services to loopback, avoid ambient credentials and user profiles, use deterministic time/randomness where needed, and cleanly distinguish unsupported, skipped, failed, and infrastructure-error outcomes.
