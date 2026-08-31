# Scorecard

MAWR evaluates successful task completion and total system cost together. No single compactness number is sufficient.

## Permanent metrics

- **Task Success:** independently verified pass/fail outcome and aggregate success rate.
- **Input Tokens / Task:** provider-reported total model input, with cached and uncached detail when available.
- **Output Tokens / Task:** provider-reported output; reasoning detail is not double-counted when included.
- **Total Tokens / Task:** provider-defined input plus output usage, classified exact or estimated.
- **Model Round-Trips / Task:** completed model requests, including requests that lead to retries or failure.
- **Peak RAM / Session:** peak measured resident memory for the defined process/session boundary.
- **CPU Time / Task:** measured CPU consumption for the defined process-tree boundary.
- **Latency / Task:** wall time from task start through independent success/failure determination.
- **Network Bytes / Task:** bytes across the documented accounting boundary.
- **Retries / Task:** repeated model, engine, action, or harness attempts, categorized by cause.

Observation payload tokens, serialization time, state size, and omitted-unit counts are diagnostic metrics, not replacements for end-to-end usage.

## Interpretation

Task success is the gate. Comparisons report both per-attempt and per-success costs so failures cannot make a mode look artificially cheap. A token reduction that materially harms success is a regression.

All metrics name the task, mode, version, model, engine, fixture, repetitions, measurement source, and exact/estimated/unavailable classification. Aggregates include spread and raw-run links under [BENCHMARKS.md](BENCHMARKS.md).

## Release use

Release criteria define acceptable thresholds before trials run. A release report must disclose failures, exclusions, and unavailable measurements and must not replace the permanent scorecard with a favorable subset.
