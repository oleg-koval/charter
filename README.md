# charter

> Enforces a code contract written as data. Every rule must be covered by a check, a review or a PR section, and a rule with no enforcement is an error.

[![CI](https://github.com/oleg-koval/charter/actions/workflows/ci.yml/badge.svg)](https://github.com/oleg-koval/charter/actions/workflows/ci.yml)

## Usage

```bash
charter
# usage: charter doctor
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
make build    # release binary at target/release/charter
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

## License

MIT - see [LICENSE](./LICENSE).
