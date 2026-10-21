# charter

> Enforces a code contract written as data. Every rule must be covered by a check, a review or a PR section, and a rule with no enforcement is an error.

[![CI](https://github.com/oleg-koval/charter/actions/workflows/ci.yml/badge.svg)](https://github.com/oleg-koval/charter/actions/workflows/ci.yml)

**A rule list is a wish until something fails when it is skipped.**

```text
# .charter/rules.toml promises that C2 ("NEVER cast.") is enforced by a check.
# The agent never wired one up. charter notices:

$ charter doctor
C2  rules.toml  promises check, but no [[check]] lists it
$ echo $?
1
```

AI agents follow long rule lists probabilistically, and an agent grading its own work is not
evidence. charter makes a skipped rule show up: rules live in `.charter/rules.toml`, how each is
proven lives in `.charter/checks.toml`, and `charter doctor` fails when a promised layer has no mapping.

**Status: pre-release, more commands land in later PRs.** `doctor`, `check` and `baseline` exist today.

## Usage

```bash
charter doctor              # reads .charter/rules.toml and .charter/checks.toml
# ok  40 rules, 7 checks, 9 artifacts

charter doctor --dir path/to/dir --json
# {"ok":false,"gaps":[{"id":"C2","file":"rules.toml","message":"promises check, but no [[check]] lists it"}]}
```

Exit 0 when clean. On any gap it prints one line per gap (`<id>  <file>  <message>`) and exits 1.
A missing or unparsable file is an error on stderr and exit 1. It reports an unsupported
`version`, duplicate rule or check ids, a rule with an empty `enforce`, a `check` or `artifact`
rule that nothing maps, and a mapping that names an unknown rule.

```bash
charter check [--dir .charter] [--on <event>] [--json]
# ast  src/a.rs:4  no-mut: mutation in domain
charter baseline [--dir .charter]
charter rubric [--dir .charter]
```

`check` runs `doctor` first and refuses to run when there are gaps. Then it runs every check
without an `on` (plus those whose `on` equals `--on`) with `sh -c`, in the current directory.
An `exit` check fails on a nonzero exit or a kill. A `sarif` check fails on each finding whose
fingerprint is not in `baseline.json` (a missing file means an empty baseline). It prints one line
per failure and exits 1, and prints nothing on success.

`baseline` runs only the `sarif` checks and writes their current findings to
`<dir>/baseline.json`, so old findings stop failing `check` and only new ones do.

`rubric` prints the review rules in file order and the required verdict table format.

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

CI runs the checks directly and dogfoods `charter doctor` on this repo's own `.charter/`.
The commands that run the three layers land in later PRs.

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
| Dogfood | `charter doctor` |
| Contract rules | ast-grep, file length |
| Secrets | gitleaks |
| PR size | `scripts/check-pr-size.sh` (200 changed lines) |
| Dependencies | Dependabot (weekly) |

## Contributing

PRs welcome. See [CONTRIBUTING.md](./CONTRIBUTING.md) and [AGENTS.md](./AGENTS.md).

## License

MIT - see [LICENSE](./LICENSE).
