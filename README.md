# rust-starter

> Opinionated Rust project starter with an enforced engineering contract: strict clippy, ast-grep rules, lefthook, gitleaks, cross-platform releases.

[![CI](https://github.com/oleg-koval/rust-starter/actions/workflows/ci.yml/badge.svg)](https://github.com/oleg-koval/rust-starter/actions/workflows/ci.yml)

## Usage

```bash
app Ada Lovelace
# Hello, Ada Lovelace!

app version
# app 0.1.0
```

## Development

Requires rustup, Node (for `npx`), lefthook and gitleaks:

```bash
brew install rustup lefthook gitleaks node
```

Then:

```bash
make hooks    # install the pre-commit gate (once)
make check    # everything CI runs
make test     # unit + end-to-end tests
make rules    # contract rules (ast-grep)
make build    # release binary at target/release/app
```

## Layout

```
src/domain/   pure logic: no IO, no mutation, no clock
src/edge/     every side effect
src/main.rs   wiring only
tests/        end-to-end tests against the built binary
rules/        ast-grep rules for the contract
.charter/     the contract as data, and how this repo proves each rule
```

## The contract

`AGENTS.md` carries a 40-rule engineering contract, and `.charter/rules.toml`
carries the same rules as data. Each rule is enforced one of three ways:

| Layer | What proves it |
|-------|----------------|
| Check | clippy lints (`Cargo.toml`), ast-grep rules (`rules/`), scripts, gitleaks |
| Review | An independent reviewer rules PASS / FAIL / N/A on every rule, with proof |
| Artifact | Required PR body sections (`.github/PULL_REQUEST_TEMPLATE.md`) |

Wiring for the `charter` CLI, which runs all three layers from `.charter/`, lands
with charter v0.1. Until then CI runs the checks directly.

## Releasing

Tag a version; GitHub Actions builds static binaries for Linux (musl), macOS and
Windows, attaches them with SHA-256 checksums, and uses `CHANGELOG.md` for notes:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## CI

| Check | Tool |
|-------|------|
| Format | `cargo fmt --check` |
| Lint | `cargo clippy -D warnings` |
| Tests | `cargo test` |
| Contract rules | ast-grep, file length |
| Secrets | gitleaks |
| PR size | `scripts/check-pr-size.sh` (200 changed lines) |
| Dependencies | Dependabot (weekly) |

## Contributing

PRs welcome. See [CONTRIBUTING.md](./CONTRIBUTING.md) and [AGENTS.md](./AGENTS.md).

## Other starters

Part of a set with shared conventions (AGENTS.md, Conventional Commits, MIT, GitHub Actions CI, Dependabot):

- [`ts-npm-starter`](https://github.com/oleg-koval/ts-npm-starter) - TypeScript / Node
- [`py-starter`](https://github.com/oleg-koval/py-starter) - Python (uv + ruff + ty)
- [`go-starter`](https://github.com/oleg-koval/go-starter) - Go
- [`rust-starter`](https://github.com/oleg-koval/rust-starter) - Rust (this repo)

## License

MIT - see [LICENSE](./LICENSE).
