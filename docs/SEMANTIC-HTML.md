# Static HTML semantic extraction: implemented M3 boundary

`mawr-semantic-html` converts bounded static HTML bytes into deterministic, ordered semantic units. It accepts the `DocumentInput` produced by `mawr-native-static`, and also exposes `HtmlDocumentSource` for deterministic fixtures and future engine adapters. Declared media types must be `text/html` or `application/xhtml+xml`; absent metadata is accepted for fixture and origin compatibility, while a declared non-HTML type is rejected. It executes no script and returns no raw DOM to an agent.

## Standards and parser baseline

The implemented subset follows the stable [Accessible Name and Description Computation 1.1 Recommendation](https://www.w3.org/TR/accname-1.1/), [WAI-ARIA 1.2 Recommendation](https://www.w3.org/TR/wai-aria-1.2/), and current [HTML Accessibility API Mappings 1.0](https://www.w3.org/TR/html-aam-1.0/). “Subset” is deliberate: MAWR does not claim browser accessibility-tree parity.

HTML is repaired with `html5ever` 0.39 into a small MAWR-owned `ego-tree` 0.11 sink. This avoids CSS selector machinery and keeps the locked parser graph within the repository's license policy. MAWR does not use `markup5ever_rcdom`, whose own package documentation describes it as an unsupported test DOM. `encoding_rs` handles BOM, HTTP `charset`, and early HTML `meta charset` labels; deterministic UTF-8 is the fallback and replacement decoding is reported.

## Produced semantics

The extractor emits the core roles for page, region, heading, text, link, form, textbox, checkbox, radio, select, option, button, table, row, cell, list, list item, and alert. Native HTML supplies implicit roles; recognized ARIA roles may override them. `role=none` and `role=presentation` suppress only the element unit. Unsupported explicit roles retain any supported native meaning and produce a notice.

Names use this bounded priority order:

1. ordered, unique `aria-labelledby` targets;
2. `aria-label`;
3. explicit or wrapping HTML `label`;
4. supported input `alt` or `value` alternatives;
5. visible text for roles whose name may come from content;
6. `title`;
7. textbox `placeholder` as a documented fallback.

Whitespace is flattened and values are UTF-8 byte bounded. `aria-describedby` supplies descriptions. Broken references, duplicate IDs and cycles are not guessed: they produce notices and an explicit ambiguous property. Password values are redacted, including from `Debug` output.

Controls expose disabled, checked, selected, expanded, required, and invalid state where the static markup supports it. Disabled fieldset inheritance honors the first-legend exception; an otherwise unselected single select exposes its first enabled option as selected. Links and form destinations resolve against the first valid HTTP(S) `base` element and then the final transport URL. Non-HTTP(S) destinations are unsupported and receive no follow affordance.

Relationships retain label, description, control, form ownership, option/select, row/table, cell/row, and list-item/list provenance. Every DOM node receives a deterministic pre-order `SourceNodeId`; this is parser provenance, not a stable action reference. M4 owns cross-state identity and conversion into stable `ElementRef` values.

## Visibility and honesty boundary

The extractor excludes `hidden`, `aria-hidden=true`, hidden inputs, script, style, template, and supported inline declarations for `display:none`, `visibility:hidden|collapse`, and `content-visibility:hidden`. It does not evaluate stylesheets, layout, generated content, shadow DOM, scripts, or live control state. A stylesheet produces an `ExternalCssVisibilityUnknown` notice so static visibility is not presented as browser-rendered truth.

## Resource and measurement boundary

Before semantic extraction MAWR enforces nonzero, capped limits for input bytes, DOM nodes, DOM depth, attributes per element, aggregate text bytes, semantic units, relationships, and notices. A violation returns a structured `ResourceLimit`; there is no silent truncation of the semantic document. The input-byte bound limits parser exposure, while post-parse DOM bounds prevent unbounded traversal. The html5ever `TreeSink` callback contract does not return fallible node creation, so the sink cannot abort at an exact node count without unwinding; M3 therefore does not represent this as a hard allocator quota.

Diagnostics record exact input bytes, parsed node count, semantic-unit count, relationship count, notice count, and wall latency. CPU time and peak memory remain explicitly unavailable until a reliable process measurement harness exists. The fixed public corpus covers small pages, tables, boilerplate, irrelevant nodes, controls, malformed HTML, ambiguity, Unicode, base URLs, and parser limits; it supports diagnostics, not a performance claim.
