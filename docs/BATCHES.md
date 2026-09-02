# Deterministic action batches: implemented M8 boundary

M8 extends the private `mawr-actions` library with bounded ordered `ActionBatch` execution. It reduces caller decision boundaries while retaining M7's expected-state, semantic, capability, and authorization checks for every action. This is not yet a wire protocol, CLI command, MCP tool, script runner, or distributed transaction.

## Batch contract

An `ActionBatch` contains one initial `expected_state`, between 1 and 64 typed actions, and one explicit failure policy. Every referenced element must belong to the expected state's session. The executor rejects a stale initial state before authorization, local mutation, or network access.

Whole-batch preflight runs against a cloned semantic state store. Local `fill`, `check`, `uncheck`, and `select` effects are simulated in order, allowing later actions such as `submit` to validate and prepare against their deterministic resulting form state. The real store is restored before execution. Preparation and caller authorization happen exactly once per item; execution reuses those prepared actions rather than silently authorizing a different operation.

Network actions form a semantic-knowledge boundary because their resulting references cannot be known before the request completes. After the first `navigate`, `follow`, network-normalized `press`, or `submit`, preflight permits only absolute `navigate` actions. Any later reference-bearing action rejects the entire batch before effects. This makes permitted suffixes independent of unknown response content.

## Failure policies and partial results

- `StopOnFailure` attempts the validated prefix through the first runtime failure and marks every remaining item as skipped due to that prior failure.
- `ContinueIndependent` may continue only the preflight-proven absolute-navigation suffix. It does not make arbitrary dependent actions independent.

A preflight failure is atomic with respect to MAWR state and network activity. Runtime network failures cannot be rolled back; `BatchOutcome` therefore contains one ordered result per item, the exact initial and final states, and failures with M7's `NotStarted`, `Requested`, or `NetworkCompleted` evidence. Retrying the same batch after a committed local prefix is rejected as stale and requires a fresh observation.

## Audit and diagnostics

Batch audit events identify item index, requested/effective action, expected state, operation, target, destination, phase, failure class, and applicable side-effect boundary. They never contain fill values or form values. Events distinguish authorization, preflight rejection, success, runtime failure, and policy skip.

Diagnostics record action, attempted, and failure counts plus preflight and execution wall latency. `decision_boundaries_avoided` is the structural value `action_count - 1`: it describes how many separate caller decisions the batch interface can replace, not a measured model-performance claim. The fixed tests compare equivalent batched and sequential form flows; task-level latency and token claims remain deferred to the common benchmark harness.

## Verification boundary

Owned loopback tests cover all-valid dependent batches, equivalent sequential behavior, stale and invalid-middle rejection, navigation boundaries, partial network completion, stop and continue policy, stale retry, per-item authorization denial, secret-safe audit/debug data, and competing batches built from one state. The existing `mawr-actions` dependency boundary remains unchanged and introduces no third-party runtime dependency.
