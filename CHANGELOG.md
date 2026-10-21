# Changelog

All notable changes documented here. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `charter doctor [--dir <path>] [--json]`: checks that every rule has every promised
  layer mapped, ids are unique and the version is supported. Crate `charter-cli`, binary `charter`.
- Initial scaffold: domain/edge split, strict clippy, ast-grep contract rules,
  lefthook, gitleaks, contract as data in `.charter/`.
