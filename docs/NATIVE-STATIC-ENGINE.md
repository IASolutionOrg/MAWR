# Native static engine: implemented M2 boundary

`mawr-native-static` is MAWR's first executable engine component. It acquires bounded HTTP documents and downloads as validated MAWR values without launching a browser or external process. It is a Rust library boundary for later parsing and runtime layers, not yet a CLI or a complete browser.

## Implemented transport

The crate provides asynchronous HTTP/1.0 and HTTP/1.1 GET and HEAD navigation, URL-encoded GET and POST form submission, HTTP and HTTPS, gzip/deflate/Brotli decoding, relative URL resolution, manual redirect handling, session-scoped cookies, cancellation, distinct connect/read/total timeouts, and explicitly authorized downloads.

Redirect processing follows 301/302/303 method rewriting and preserves the request for 307/308. Every hop is parsed, authorized, resolved, and connected independently. Redirect loops, missing locations, excessive redirects, invalid URLs, unsupported schemes, TLS failures, timeouts, cancellation, and resource limits remain distinguishable typed failures. Raw transport errors, response bodies, cookie values, form values, and local download paths are not exposed by `Debug` output.

Capabilities are reported from the engine configuration. HTTP(S), navigation, redirects, cookies, forms, and downloads carry their actual bounds. JavaScript, parsing, semantic state, CSS layout, visual rendering, screenshots, storage profiles, media, and external-engine fallback are explicitly unsupported or not implemented.

## Destination and TLS policy

The default destination policy permits ports 80 and 443 only and accepts only globally routable IPv4 addresses or currently assignable IPv6 global-unicast addresses outside known special-purpose ranges. It rejects loopback, private, link-local, multicast, documentation, benchmarking, translation, 6to4, metadata-style, and reserved forms. URL credentials are rejected.

The policy is evaluated after DNS resolution on every redirect hop. All returned addresses must be allowed and fit the DNS-address budget; the approved set is pinned into the HTTP client and the connected peer is checked against that exact set. This fail-closed rule prevents a mixed public/private answer from being treated as public. Ambient HTTP proxy settings are disabled and proxy support is not part of M2.

Loopback fixtures use an explicit single-port loopback policy. Other private addresses or non-standard ports require an explicit address-and-port allowlist at engine construction; allowing a hostname alone is intentionally insufficient.

HTTPS uses rustls, platform trust roots by default, TLS 1.2 or newer, and normal hostname verification. Tests can instead provide an explicit DER certificate set. Deterministic TLS integration tests use a repository fixture CA and a localhost certificate; the committed fixture key is test-only and grants no production authority.

## Resource limits

Default per-operation or per-session limits are:

| Resource | Default |
| --- | ---: |
| Decoded response body | 8 MiB |
| Decoded download | 64 MiB |
| Response headers | 64 KiB and 256 fields |
| Encoded form | 256 KiB and 256 fields |
| Accepted `Set-Cookie` bytes per session | 64 KiB |
| Redirects | 10 |
| DNS addresses per hop | 16 |
| Connect/read/total timeout | 10/15/60 seconds |

Configuration rejects zero and excessive values. Response and download limits apply after content decoding, so a small compressed payload cannot expand without bound. The cookie budget conservatively counts accepted `Set-Cookie` header bytes for the session rather than trying to estimate internal jar allocation; once exhausted, further cookies are rejected and the session remains in an explicit resource-limit state. Keeping that state persistent makes concurrent use fail closed rather than attributing one request's overflow to another.

## Download boundary

A download requires an existing directory to be canonicalized into `DownloadPolicy`, an explicit cross-platform-safe filename, and a byte limit. The engine ignores server-suggested filenames, revalidates that the canonical root has not changed, creates a new file without overwriting an existing one, streams within the decoded-byte bound, and removes partial output after cancellation or failure. The root must be controlled by the caller and protected from concurrent local replacement; M2 does not claim an OS directory-handle sandbox against a malicious local actor racing the filesystem check.

## Diagnostics and evidence

Transport diagnostics report exact request and redirect counts, exact decoded body bytes, exact zero retries, and wall-clock latency from a runtime counter. Exact wire bytes are unavailable from the selected client boundary and CPU time is not measured, so both remain explicitly unavailable rather than estimated. These diagnostics are implementation evidence, not a task-efficiency benchmark or performance claim.

The deterministic suite covers methods, URL/form encoding, redirect semantics and loops, cookies and session isolation, compression and bounds, headers, timeouts, cancellation, safe downloads and cleanup, destination denials, secret-safe errors, local TLS trust, and negative certificate validation. `cargo xtask verify` additionally rejects native-engine dependencies or source paths that introduce browser, external-process, blocking Reqwest, HTTP/2, or ambient-proxy fallback.

The destination classification follows the IANA [IPv4 special-purpose registry](https://www.iana.org/assignments/iana-ipv4-special-registry/) and [IPv6 special-purpose registry](https://www.iana.org/assignments/iana-ipv6-special-registry/). Redirect semantics follow [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110), URL resolution follows [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986), and session cookie behavior follows [RFC 6265](https://www.rfc-editor.org/rfc/rfc6265).
