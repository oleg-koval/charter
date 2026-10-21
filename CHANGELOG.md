# Changelog

All notable changes documented here. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
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
