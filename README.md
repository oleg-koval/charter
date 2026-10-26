# charter

> Enforces a code contract written as data. Every rule must be covered by a check, a review or a PR section, and a rule with no enforcement is an error.

[![CI](https://github.com/oleg-koval/charter/actions/workflows/ci.yml/badge.svg)](https://github.com/oleg-koval/charter/actions/workflows/ci.yml)

**A rule list is a wish until something fails when it is skipped.**

Read the [Engineering Testament](#engineering-testament): the 40 rules Charter exists to uphold.

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

A repo may lack tooling for a shared `check` rule. Waive it in `checks.toml`; `doctor` prints a
`waived` line and fails on a blank reason, unknown rule, stale waiver, duplicate waiver or a rule that does not promise `check`. `review` and `artifact` still apply. The `waived` list is every declared waiver, valid or not.

```toml
[[waive]]
rule = "C1"
reason = "no ast-grep rule for this language yet"
```

**Status: pre-release, more commands land in later PRs.** `doctor`, `check` and `baseline` exist today.

## Install

Install the latest release on Linux or macOS:

```sh
curl -fsSLO https://raw.githubusercontent.com/oleg-koval/charter/main/install.sh
sh install.sh
```

Set `CHARTER_VERSION` to install a specific release (with or without the leading `v`), or
`CHARTER_INSTALL_DIR` to choose the destination directory. The installer verifies the
archive's SHA-256 checksum before extracting the binary.

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
charter check [--dir .charter] [--on <event>] [--changed <base>] [--json]
# ast  src/a.rs:4  no-mut: mutation in domain
charter baseline [--dir .charter]
charter rubric [--dir .charter] [--json]
charter verify-review <file> [--dir .charter] [--json]
charter verify-pr <body-file> [--dir .charter] [--json]
charter hook [--dir .charter] < hook-input.json
charter init [--dir .charter]
```

`check` runs `doctor` first and refuses to run when there are gaps. Then it runs every check
without an `on` (plus those whose `on` equals `--on`) with `sh -c`, in the current directory.
An `exit` check fails on a nonzero exit or a kill. A `sarif` check fails on each finding whose
fingerprint is not in `baseline.json` (a missing file means an empty baseline). It prints one line
per failure and exits 1, and prints nothing on success.
`--changed <base>` filters SARIF findings to files in `git diff --name-only <base>...HEAD`; uncommitted edits are not included, and exit checks still run normally.

`baseline` runs only the `sarif` checks and writes their current findings to
`<dir>/baseline.json`, so old findings stop failing `check` and only new ones do.

`rubric` prints the review rules in file order and the required verdict table format.
`verify-review` rejects missing, duplicate, unknown, invalid, or unevidenced review rows.
`verify-pr` requires each configured artifact heading to contain content outside HTML comments.
`hook` reads `tool_input.file_path`, prints matching SARIF findings as Claude Code PostToolUse JSON, and always exits successfully.
`init` writes the embedded policy and placeholder checks into a new charter directory; it refuses existing files.

## GitHub Action

Installs charter, runs `charter doctor`, then runs the checks mapped in `.charter/checks.toml`:

```yaml
- uses: actions/checkout@v4
- uses: oleg-koval/charter@v0.2.0
  with:
    version: v0.2.0
    dir: .charter
    pr-body-file: path/to/pr-body.md # optional
```

Omit `pr-body-file` to skip `charter verify-pr`. The action supports Linux and macOS, matching the installer.

From v0.2.0, the action accepts `report: github` to add
an aviation-style Actions summary and native failure annotations. The report
leads with the cause, who should act, concrete next steps and the recovery
condition. It lists check results and keeps raw findings, rule exceptions and
rerun commands in expandable details. Unknown infrastructure failures remain
unconfirmed until someone inspects the failed step. Missing tools
or invalid results remain failures, never passes. This reporting is action-owned;
consumer repositories do not copy its scripts. Existing release tags retain
their existing behavior until a new action revision is selected.

Set `checks: 'false'` with `pr-body-file` for a fast PR-description job. The
default remains `checks: 'true'`; adding a PR body does not skip code checks.
`dir` applies to every station. A report verifies only the selected checks,
not independent review or merge readiness. Pin a reviewed action commit when
adopting these inputs; `version` continues to select the CLI binary separately.

### PR flight report

Charter also owns the PR comment publisher at `oleg-koval/charter/report`.
It combines current-head check results into one bot-owned comment, with station
signals, findings, run links and a waiver register. It distinguishes policy
failures from infrastructure errors, pending, cancelled and missing runs.

Call it from a separate workflow on the default branch:

```yaml
name: Charter report
on:
  workflow_run:
    workflows: [Charter, Charter PR]
    types: [completed]
permissions:
  contents: read
  actions: read
  checks: read
  pull-requests: write
concurrency:
  group: charter-report-${{ github.event.workflow_run.head_repository.id }}-${{ github.event.workflow_run.head_branch }}
  cancel-in-progress: false
jobs:
  report:
    if: github.event.workflow_run.event == 'pull_request'
    runs-on: ubuntu-latest
    steps:
      - uses: oleg-koval/charter/report@v0.2.0
```

Use the same reviewed revision as the reporting action, and enable `report: github`
in both source workflows. Their filenames default to `charter.yml` and
`charter-pr.yml`; override with `code-workflow` and `pr-workflow`. The publisher
reads GitHub results and never checks out PR code or downloads its artifacts.
Consumers own workflow triggers, runner routing and permissions; Charter owns
the report implementation. A body edit requires a fresh PR-sections event:
rerunning an old event still checks its original body snapshot.

`version` defaults to the release that matches the action ref, so a pinned action does not pull a newer binary. The checks run as shell commands from your repository, so every tool they call must exist on the runner, and the action must not run under `pull_request_target` or any event that checks out untrusted code while secrets are in scope.

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

## Engineering Testament

These rules are not negotiable. A conflict is raised before code is written;
a rule is never broken silently or worked around.

The [canonical contract](.charter/rules.toml) carries these 40 rules as data;
[AGENTS.md](AGENTS.md) carries the agent instructions. Each rule is enforced
through the layers named in the contract:

| Layer | What proves it |
|-------|----------------|
| Check | clippy lints (`Cargo.toml`), ast-grep rules (`rules/`), scripts, gitleaks |
| Review | An independent reviewer rules PASS / FAIL / N-A on every rule, with proof |
| Artifact | Required PR body sections (`.github/PULL_REQUEST_TEMPLATE.md`) |

### Code

1. NEVER mutate in domain code: no `swap!`/`reset!`, volatiles, `ref.current =`, `push`/`splice`, property or `let` reassignment. At most one atom per stateful edge, changed only through `swap!` with a pure function.
2. NEVER cast: no `as X`, `<X>value`, non-null `!`. Narrow with type guards or schemas.
3. NEVER use `any` (TS `any`, Malli `:any`). Validate at the edges, keep maps open inside, never reject unknown keys deep in the code.
4. Domain code NEVER imports HTTP, routes, schemas, api or contracts. No god modules.
5. Side effects (DB, HTTP, clock, IO) live ONLY in a thin edge layer. Policy stays apart from mechanism. Nothing hides cost or control flow.
6. NEVER break users. Evolve by adding; require less, promise more. A breaking change gets a new name.
7. One concern per PR, under about 200 changed lines. Every commit builds and passes tests.
8. NEVER hand-edit generated files. Change the source or the generator, then regenerate.
9. Fail open outward, loud inward: the caller gets the fallback, and every fallback is logged and counted in a metric.
10. NEVER commit secrets or passwords, in code, commits or PR bodies.

### Thinking

11. Write the problem and its constraints down before coding.
12. Design the data structures and their relationships first.
13. Simple over easy: never braid concerns together.
14. Clarity over cleverness: a trick that needs a comment gets rewritten.
15. Domain rules are small, named, pure functions over plain data.
16. Reuse before writing.
17. When unsure, prototype both ways and compare.
18. Every rule serves correctness, testability, readability or speed. A conflict is raised, never bent silently.

### Data

19. Data is an immutable fact with its time. History stays append-only where it matters.
20. Plain open maps with namespaced keys; no wrapper objects.
21. Put knowledge in data: table- or map-driven dispatch over `cond` chains.
22. Remove a special case by reshaping the data or the code, never by adding another `if`.

### Functions and interfaces

23. One concern per function. 25 lines is a smell, single-use code may stay inline, never more than 3 levels of nesting.
24. Assert invariants in domain functions; fail loudly in dev and test.
25. Least surprise: the same names, status codes, error shapes and paging everywhere.
26. Every decision path is observable through structured logs and a metric. Quiet on success, loud on the unexpected.
27. A comment states only a non-obvious "why", never what the code does.

### Performance

28. Programmer time beats machine time. Optimize only measured hot paths.
29. Every hot path has a latency budget. Every performance claim carries a profile or benchmark.

### Change and history

30. Make the smallest change that does the job. Fix only findings worth fixing.
31. Violations outside the diff are named as out of scope, never fixed silently.
32. A failing-CI fix is the smallest change that turns it green. Old code is grandfathered.
33. A big refactor is a series of PRs, with a pure move first.
34. A commit message says why the change exists and what breaks without it.
35. NEVER force-push, and NEVER rebase a pushed branch.
36. A regression comes before any feature. Undo it at once, flag off first where a flag exists.

### Proof and process

37. Every claim is backed by a diff, test output, permalink or run URL.
38. No merge without an independent review of the diff file only.
39. Every PR body opens with a plain "TL;DR (for humans)".
40. Hooks act on output and never block. Starter repos stay generic templates.

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
