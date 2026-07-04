# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in `firebase-admin-rs` — especially
anything in the token verification path (`src/auth/id_token/`,
`src/auth/session_cookie/`) — please **do not open a public GitHub issue**.

Instead, report it privately via [GitHub Security Advisories](../../security/advisories/new)
for this repository. Please include:

- A description of the vulnerability and its potential impact
- Steps to reproduce, or a proof-of-concept if possible
- The affected version(s)

We aim to acknowledge reports within 5 business days and to release a fix or
mitigation as quickly as possible depending on severity.

## Supported Versions

Until a `1.0.0` release, only the latest published `0.x` version receives
security fixes. After `1.0.0`, this policy will be updated to reflect a
supported version range.
