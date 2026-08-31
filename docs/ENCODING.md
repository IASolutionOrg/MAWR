# Encoding architecture

JSON is an external representation, not MAWR's internal model.

```text
Typed Rust Observation
          |
          v
ObservationEncoder
   |         |          |               |
   v         v          v               v
compact    TOON       compact text    model-specific
JSON       (future)   (future)        (future)
```

## Boundary

Engines, semantic extraction, state and diffing, relevance, budgeting, and action execution operate on MAWR-owned typed data. An encoder consumes a complete typed observation and produces a transport representation plus measurement metadata. Adding an encoder must not change those upstream layers.

Decoders for action input follow the same rule: boundary syntax is validated and translated into typed actions before authorization or execution.

## MVP encoding

Compact JSON is the planned first encoding because it is widely supported and debuggable. No encoder or stable public wire schema exists yet. Compact means avoiding redundant field names and payloads only after semantic selection; it does not mean weakening type meaning, deleting necessary state, or truncating serialized bytes.

The versioned encoding must preserve state IDs, stable references, semantic roles, action affordances, meaningful changes, omissions, capability/failure information, and metric classification.

## Token measurement

Observation payload tokens will be measured locally with an identified tokenizer when possible and otherwise marked estimated with the method recorded. These diagnostic tokens are distinct from total provider-reported input and output usage. Selection operates on complete units and never truncates encoded bytes.

## Future encodings

TOON, compact text, model-specific encodings, and automatic selection are post-MVP research. Adoption requires same-task measurements of token count, task success, model comprehension, latency, and serialization overhead. No encoding is preferred merely because a standalone sample is shorter.
