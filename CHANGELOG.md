# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

This file is maintained automatically by [release-plz](https://release-plz.dev/)
from [Conventional Commits](https://www.conventionalcommits.org/) — see
`CONTRIBUTING.md`.

## [Unreleased]

## [0.1.0](https://github.com/0xrustdev/Firebase-Admin-SDK/releases/tag/v0.1.0) - 2026-07-04

### Added

- implement live-mode user management and session cookie verification
- scaffold firebase-admin-rs v0.1.0 Auth MVP

### Fixed

- select rust_crypto backend for jsonwebtoken 10
- address code review findings and harden release pipeline

### Other

- *(deps)* bump jsonwebtoken from 9.3.1 to 10.4.0
- Merge pull request #4 from 0xrustdev/dependabot/cargo/x509-parser-0.18.1
- Merge pull request #2 from 0xrustdev/dependabot/github_actions/actions/setup-node-6.4.0
- *(deps)* bump actions/setup-node from 4.4.0 to 6.4.0
- point repository URLs at the real GitHub repo
- fix stale roadmap contradicting the Features section
- cover JWKS TTL expiry, document ADC non-functional status

### Security

- document accepted RSA timing-sidechannel risk (RUSTSEC-2023-0071)
