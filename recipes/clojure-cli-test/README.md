# Clojure CLI with a test alias

This recipe wires `clojure -M:test` into Charter. It requires Java, Clojure CLI,
project dependencies, and an explicitly configured `:test` alias in `deps.edn`.
That alias must select a test runner, load the intended namespaces, terminate,
and return a nonzero exit code on failed assertions. `:test` is this recipe's
chosen convention, not a universal Clojure alias. Nothing is installed or detected.

From the project root, using a reviewed local copy of this recipe:

```sh
charter init --checks-from /path/to/recipes/clojure-cli-test/checks.toml
charter check
```

Leiningen, Babashka, exec aliases, and differently named aliases need a customized
command. Successful alias execution proves that command only, not exhaustive
namespace/AOT compilation, meaningful coverage, source rules, or independent review.
If the project requires a separate build or AOT step, you must add its real build
command before `clojure -M:test` or as another C7 check before treating C7 as
covered. The recipe cannot choose that project-specific command for you.

`check` remains unsuccessful until real source and secret checks replace
`configure-enforcement` for its listed rules. Prove compliant and violating
fixtures for those tools. Do not replace the barrier with `true` or broad waivers.
`doctor` validates mappings; it does not prove the alias runs meaningful tests.

Existing configuration is never overwritten. For an existing project, add
`--dir .charter-review` to init and check, then review the difference. Commands
run from the invocation directory. Imported configuration is project-owned;
Fleet can distribute pinned recipe sources and README, but updates do not
upgrade existing imports automatically.

The integration test uses a fixture executable to verify argv, working directory,
nonzero status, and missing-tool failure. It does not execute Clojure. Before
adoption, run the actual alias with passing and deliberately failing tests.
