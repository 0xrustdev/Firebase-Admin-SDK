# Contributing to firebase-admin-rs

Thanks for your interest in contributing! This project aims to be a
well-documented, well-tested, community-maintained Firebase Admin SDK for
Rust — contributions of all sizes are welcome, from typo fixes to new
features.

## Getting started

1. Fork the repository and clone your fork.
2. Install the stable Rust toolchain (see `rust-toolchain.toml`).
3. Run the test suite: `cargo test --all-features`.
4. For changes touching user management or session cookies, install the
   [Firebase CLI](https://firebase.google.com/docs/cli) and run the local
   emulator: `firebase emulators:start --only auth`, then run the
   emulator-backed integration tests with `FIREBASE_AUTH_EMULATOR_HOST` set
   (see `tests/emulator_*.rs`).

## Project structure

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the module layout and the
reasoning behind it — in particular, why `src/core/` exists separately from
`src/auth/` even though there is currently only one service.

## Before opening a pull request

- Run `cargo fmt`.
- Run `cargo clippy --all-targets --all-features -- -D warnings` and fix any
  warnings.
- Run `cargo test --all-features` and `cargo test --doc --all-features`.
- Add or update tests for any behavior change. Token-verification logic in
  particular should include negative-case coverage (expired, wrong audience,
  tampered signature, etc.) — see `src/auth/id_token/verifier.rs` for the
  existing pattern.
- Add a doc comment (`///`) to any new public item; `cargo doc --all-features`
  should build without warnings.

## Commit messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/)
(`feat:`, `fix:`, `docs:`, `chore:`, etc.) so that
[release-plz](https://release-plz.dev/) can generate accurate changelogs and
version bumps automatically. Please follow this convention in your PR title
and/or commits.

## Branching model

This project uses trunk-based development: `main` is always releasable, and
changes land via short-lived `feat/*`/`fix/*` branches merged through a
reviewed, CI-gated pull request. There is no long-lived `dev` branch.

## Reporting bugs / requesting features

Please use the issue templates under `.github/ISSUE_TEMPLATE/`. For security
vulnerabilities, see [`SECURITY.md`](SECURITY.md) instead of opening a public
issue.

## Code of Conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
