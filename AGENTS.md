# AGENTS.md

Instructions for AI coding agents (Claude Code, Codex, Cursor, Copilot).

## Setup

Requires Rust (the toolchain is pinned in `rust-toolchain.toml`), Node (for `npx`,
which runs ast-grep), [lefthook](https://lefthook.dev/) and
[gitleaks](https://github.com/gitleaks/gitleaks). If missing:

```bash
brew install rustup lefthook gitleaks node
rustup-init -y
```

## Git hooks

Install once:

```bash
make hooks
```

The pre-commit gate runs fmt, clippy, tests, the ast-grep rules, the 300-line
file check and gitleaks. It blocks a bad commit on purpose. Contract rule 40
("hooks never block") is about agent tool hooks, not this gate.
`git commit --no-verify` is rejected in code review.

## Commands

- Everything CI runs: `make check`
- Tests: `make test`
- Lint: `make lint` (clippy, warnings are errors)
- Contract rules: `make rules` (ast-grep, see `rules/`)
- Format: `make fmt`
- Build: `make build`

## Layout

```
src/domain/   pure functions over plain data: no IO, no mutation, no clock
src/edge/     the only side effects: args, stdout, stderr, exit codes
src/main.rs   wiring only
tests/        end-to-end tests against the built binary
rules/        ast-grep rules that enforce the contract mechanically
.charter/     the contract as data (rules.toml) and how this repo proves it (checks.toml)
```

## How the contract is enforced

Every rule in `.charter/rules.toml` names how it is enforced:

| enforce | Means |
|---|---|
| `check` | A command in `.charter/checks.toml` proves it: clippy lints in `Cargo.toml`, ast-grep rules in `rules/`, scripts in `scripts/`. |
| `review` | The independent reviewer rules PASS, FAIL or N/A on it, with proof, for every PR. |
| `artifact` | The PR body carries its section (see `.github/PULL_REQUEST_TEMPLATE.md`). |

A lint exception uses `#[expect(clippy::lint, reason = "...")]`, never `#[allow]`:
it states why, and it fails once the exception is no longer needed.

## Conventions

- Errors are values: return `Result` with a domain error enum. No `unwrap`,
  `expect` or `panic!` outside tests.
- No `as` casts: use `From`/`TryFrom`.
- Tests: end-to-end tests in `tests/` exercise the binary; unit tests sit next to
  pure functions with real branching.
- Doc comments only on public items, and only where the name is not enough.

## Don't

- Don't add dependencies without discussion. Std first.
- Don't commit without `make check` passing.
- Don't edit `.charter/rules.toml` here: it is shared across repos.

## Engineering contract

These rules are not negotiable. A conflict with a rule is raised before code is
written; a rule is never broken silently and never worked around.

### I. Code
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

### II. Thinking
11. Write the problem and its constraints down before coding.
12. Design the data structures and their relationships first.
13. Simple over easy: never braid concerns together.
14. Clarity over cleverness: a trick that needs a comment gets rewritten.
15. Domain rules are small, named, pure functions over plain data.
16. Reuse before writing.
17. When unsure, prototype both ways and compare.
18. Every rule serves correctness, testability, readability or speed. A conflict is raised, never bent silently.

### III. Data
19. Data is an immutable fact with its time. History stays append-only where it matters.
20. Plain open maps with namespaced keys; no wrapper objects.
21. Put knowledge in data: table- or map-driven dispatch over `cond` chains.
22. Remove a special case by reshaping the data or the code, never by adding another `if`.

### IV. Functions and interfaces
23. One concern per function. 25 lines is a smell, single-use code may stay inline, never more than 3 levels of nesting.
24. Assert invariants in domain functions; fail loudly in dev and test.
25. Least surprise: the same names, status codes, error shapes and paging everywhere.
26. Every decision path is observable through structured logs and a metric. Quiet on success, loud on the unexpected.
27. A comment states only a non-obvious "why", never what the code does.

### V. Performance
28. Programmer time beats machine time. Optimize only measured hot paths.
29. Every hot path has a latency budget. Every performance claim carries a profile or benchmark.

### VI. Change and history
30. Make the smallest change that does the job. Fix only findings worth fixing.
31. Violations outside the diff are named as out of scope, never fixed silently.
32. A failing-CI fix is the smallest change that turns it green. Old code is grandfathered.
33. A big refactor is a series of PRs, with a pure move first.
34. A commit message says why the change exists and what breaks without it.
35. NEVER force-push, and NEVER rebase a pushed branch.
36. A regression comes before any feature. Undo it at once, flag off first where a flag exists.

### VII. Proof and process
37. Every claim is backed by a diff, test output, permalink or run URL.
38. No merge without an independent review of the diff file only.
39. Every PR body opens with a plain "TL;DR (for humans)".
40. Hooks act on output and never block. Starter repos stay generic templates.

