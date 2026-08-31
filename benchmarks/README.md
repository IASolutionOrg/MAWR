# Benchmark suite layout

This directory is reserved for the future public benchmark suite. It contains documentation only during Phase 0. The normative fairness, baseline, trial, token-accounting, Test Mode, and reporting rules remain defined only in [../docs/BENCHMARKS.md](../docs/BENCHMARKS.md).

## Planned organization

```text
benchmarks/
  tasks/       versioned canonical task definitions
  fixtures/    deterministic sites and initial-state data
  drivers/     baseline, MAWR, and ablation adapters
  schemas/     task and result schemas
  results/     only sanitized, intentionally published evidence
  reports/     reports generated from raw published data
```

Directories should be created only when real implementation content exists. Generated local results will live under ignored `target/mawr-bench/`, not in this source directory. Raw machine-readable runs are primary; Markdown reports are generated from them.

## Adding a benchmark after tooling exists

A contribution will add a versioned task, deterministic fixture and reset, independent success checker, limits and allowed capabilities, then demonstrate the task in Reference Full-State Baseline and MAWR modes under the same harness. Driver-specific instructions and tool schemas must be stored as artifacts and charged to model input.

Fixtures must run locally without credentials or private services. Model-backed execution remains explicit and cost/network opted-in. Public contributions cannot contain private prompts, authenticated site data, secrets, or unsanitized traces.

## Future execution

No benchmark command exists yet. The planned cross-platform verification entrypoint and future benchmark subcommands are documented in [../docs/DEVELOPMENT.md](../docs/DEVELOPMENT.md). Contributor instructions must be updated only when those commands are implemented and validated.
