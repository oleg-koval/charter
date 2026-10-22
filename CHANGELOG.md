# Changelog

All notable changes documented here. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] - 2026-10-22

### Added
- `[[waive]]` in `checks.toml` declares a `check` rule the repo has no tooling for (`rule`, `reason`). `doctor` prints each as a `waived` line and fails on a blank reason, unknown rule, stale waiver, duplicate waiver or a rule that does not promise `check`. `doctor --json` adds a `waived` array.

## [0.1.1] - 2026-10-22

### Fixed
- `install.sh` requested `charter-<target>.tar.gz.sha256`, but releases publish `charter-<target>.sha256`, so every install failed with a 404. The action now defaults to this release.

## [0.1.0] - 2026-10-22

### Added
- `install.sh` (checksum-verified installer) and a composite GitHub Action (`oleg-koval/charter@v0.1.0`) that runs `doctor`, `check` and, optionally, `verify-pr`.
- `charter init [--dir <path>]`: writes starter policy and doctor-valid placeholder checks.
- `charter hook [--dir <path>]`: reports SARIF findings for the edited file as Claude Code PostToolUse JSON without blocking.
- `charter check --changed <base>`: limits SARIF findings to files changed since a base revision.
- `charter verify-pr <body-file> [--dir <path>] [--json]`: checks required artifact sections.
- `charter verify-review <file> [--dir <path>] [--json]`: checks review coverage and evidence.
- `charter rubric [--dir <path>] [--json]`: prints every review rule and the required verdict table.
- `charter check [--dir <path>] [--on <event>] [--json]`: runs the mapped checks; a failing exit
  check or a SARIF finding not in `baseline.json` fails. Refuses to run when `doctor` has gaps.
- `charter baseline [--dir <path>]`: writes the current SARIF findings to `baseline.json`.
- `charter doctor [--dir <path>] [--json]`: checks that every rule has every promised
  layer mapped, ids are unique and the version is supported. Crate `charter-cli`, binary `charter`.
- Initial scaffold: domain/edge split, strict clippy, ast-grep contract rules,
  lefthook, gitleaks, contract as data in `.charter/`.

### Fixed
- `charter hook` matches a file the editor reports through a symlinked directory (macOS `/tmp`, `/var`).
