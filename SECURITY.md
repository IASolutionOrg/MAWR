# Security policy

This file defines vulnerability reporting and responsible disclosure. MAWR's technical trust boundaries and threat model are canonical in [docs/SECURITY-MODEL.md](docs/SECURITY-MODEL.md).

## Supported versions

MAWR has no runtime release. No version is currently supported for production use. This policy will be updated before the first release.

## Reporting a vulnerability

Do not publish vulnerability details in a public issue, discussion, benchmark trace, or pull request.

Use the repository host's private vulnerability-reporting feature when it is available. If no private reporting channel is published, open a public issue containing only a request for a private security contact and no technical details, exploit steps, secrets, or affected data.

Include, through the private channel:

- the affected revision or version;
- a concise impact statement;
- reproduction steps or a minimal proof of concept;
- relevant configuration and platform details;
- suggested mitigation, if known.

Maintainers should acknowledge receipt, establish a private communication channel, validate impact, coordinate a fix and disclosure timeline, and credit the reporter when requested and appropriate. Do not test against systems or data you do not own or have permission to access.
