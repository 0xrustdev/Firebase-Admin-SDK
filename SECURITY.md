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

## Supply-chain and release security

- **CI/CD**: every third-party GitHub Action used in `.github/workflows/` is
  pinned to an immutable commit SHA (not a mutable version tag), so a
  hijacked or re-tagged upstream Action cannot silently inject code into a
  workflow run. Workflows default to `permissions: contents: read` and only
  the `release-plz` job — the one that holds `CARGO_REGISTRY_TOKEN` — is
  granted write access, scoped to just that job.
- **Dependency auditing**: `cargo-deny` (see `deny.toml`) checks every
  dependency against the RUSTSEC advisory database and an explicit license
  allowlist on every PR, and again on a weekly schedule
  (`.github/workflows/scheduled-audit.yml`) to catch newly-disclosed
  advisories in dependencies that haven't otherwise changed.
- **Release integrity**: each release publishes a `SHA256SUMS.txt` alongside
  the packaged crate on its GitHub Release, so downstream users can verify
  they received the exact bytes that were published. (SHA-1 is not used
  anywhere in this project's integrity tooling — it has been cryptographically
  broken since 2017; all checksums here are SHA-256.)
- **Credential hygiene**: `core::ServiceAccountKey` has a hand-written
  `Debug` implementation that redacts the private key field, so an errant
  `{:?}`/log statement involving a loaded service account cannot leak key
  material.

## Accepted risks

- **RUSTSEC-2023-0071** (Marvin Attack: RSA timing sidechannel), via the
  `rsa` crate pulled in transitively by `jsonwebtoken`'s `rust_crypto`
  backend. No upstream fix exists; RustCrypto/RSA has not shipped a
  constant-time mitigation. Explicitly allowed in `deny.toml` after
  assessing scope: ID token and session cookie *verification* (the majority
  of this crate's RSA usage) only touches the public key and is unaffected,
  since there's no secret-dependent timing to attack. Custom token and
  session cookie *signing* does use the private key and is theoretically in
  scope, but exploitation requires an attacker able to trigger many signing
  operations against a running service and measure response timing with
  high precision over a real network — a hard, low-practical-likelihood
  attack. Switching to jsonwebtoken's `aws_lc_rs` backend would eliminate
  this but adds a C/cmake build dependency; revisit this tradeoff if
  RustCrypto ships a fix or if a future feature exposes signing behind an
  unauthenticated, high-throughput endpoint.
