# Contributing to MAWR

MAWR is currently pre-alpha. The Rust workspace, deterministic repository verification, and typed core contracts exist, but runtime behavior does not; proposals must distinguish architectural contracts from implemented behavior.

## Before changing a contract

Use [docs/README.md](docs/README.md) to find the single owner of the concern. Discuss changes that affect engine boundaries, agent-visible behavior, security boundaries, compatibility claims, benchmark fairness, or public data formats before implementation. Update the canonical document rather than copying its content elsewhere.

Every proposal must preserve the project constraints in [docs/PRINCIPLES.md](docs/PRINCIPLES.md), especially the Chromium prohibition, native static engine requirement, and task-success-first efficiency rule.

## Change quality

- Keep changes narrow and explain the machine workload they serve.
- Do not add dependencies, compatibility layers, or browser features without evidence.
- Do not make performance or compatibility claims without reproducible results.
- Keep fixtures deterministic and public; never commit credentials, authenticated content, private prompts, or unsanitized traces.
- Include tests appropriate to the affected contract once an implementation exists.
- Preserve cross-platform terminal execution.

## Verification

Install a stable Rust toolchain with the `rustfmt` and `clippy` components, then run:

```text
cargo xtask verify
```

This is the canonical deterministic, credential-free repository check. It compiles the workspace, enforces the dependency-free core boundary, checks formatting and lints, runs tests, and validates local documentation links. See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for the toolchain policy, individual commands, and the boundary between implemented and planned verification.

## Conduct and review

Be precise, respectful, and evidence-driven. Review prioritizes correctness, security, reproducibility, interoperability, and maintainability over feature count. Maintainers do not auto-merge changes, and contributors must not assume permission to create branches, commits, or remote changes on a user's behalf.
