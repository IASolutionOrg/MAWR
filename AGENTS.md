# Agent instructions

MAWR means Machine-Aware Web Runtime. These rules apply to coding agents and automated contributors in public clones and maintainer workspaces.

## Non-negotiable rules

- Rust is the primary implementation language.
- Chromium, Chrome, Blink, Electron, and hidden Chromium or Playwright-Chromium fallbacks are forbidden.
- The native static engine is mandatory. External engines are optional, explicit, and replaceable; their types must not enter MAWR-owned contracts.
- JSON is an external encoding. Internal domain types must not depend on JSON shapes.
- Keep dependencies minimal and justified.
- Benchmark claims require reproducible measurements under [docs/BENCHMARKS.md](docs/BENCHMARKS.md).
- Tests and verification must remain runnable from ordinary terminal environments on Windows, Linux, and macOS.
- Do not auto-merge, create branches or worktrees automatically, or commit or push unless the user explicitly requests the exact action.
- Do not casually rewrite architectural contracts. When durable behavior changes, update its canonical document and affected tests together.
- Do not duplicate documentation. Follow the public source-of-truth map in [docs/README.md](docs/README.md).
- Public benchmark tooling, fixtures, raw results, and methodology must remain reproducible and must not contain secrets or private data.

## Local maintainer context

If `.internal/` exists in the current local workspace, read `.internal/PROJECT-STATE.md`, `.internal/MVP.md`, `.internal/MVP-IMPLEMENTATION-PLAN.md`, and `.internal/TECHNICAL-DECISIONS.md` when relevant. Their absence must never block an ordinary public contribution; public contracts are complete without them.
