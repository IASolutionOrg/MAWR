# Vision

Browsers turn the web into pixels for humans. MAWR turns the web into actionable state for machines.

MAWR exists because a human-oriented rendering pipeline is often an expensive and indirect interface for an agent. The runtime should preserve the web's useful state and action semantics while avoiding work that does not help a machine complete a task.

```text
HTML / JavaScript / Network
            |
            v
machine-oriented page state
            |
            v
semantic representation
            |
            v
relevance selection and token budget
            |
            v
agent observation and action
```

The first product proof is deliberately narrower than web compatibility in general: on supported deterministic tasks, an agent must complete the same task as a defined reference baseline while using substantially less model context and fewer model decisions. Independent machine-verifiable checks determine success.

MAWR optimizes the complete task, not a serialization sample. Its primary efficiency metric is total model tokens per successfully completed task. The permanent scorecard also covers success rate, model round-trips, memory, CPU, latency, network bytes, and retries. A token reduction that materially harms task success is a regression.

Rendering is not the primary output. Human-facing browser completeness, consumer UI, DRM, full media playback, printing, extension ecosystems, complete animation, unnecessary compositing, and full WebRTC may remain unsupported indefinitely unless real machine workloads justify them.
