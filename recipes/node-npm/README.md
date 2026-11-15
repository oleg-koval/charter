# Node/npm checks

This recipe wires `npm run build && npm test` into Charter. It requires Node/npm,
already-installed project dependencies, and `build` and `test` package scripts
that terminate and fail on errors. For TypeScript, the build script must run the
project's intended compiler checks. Nothing is installed or autodetected.

From the project root, using a reviewed local copy of this recipe:

```sh
charter init --checks-from /path/to/recipes/node-npm/checks.toml
charter check
```

`doctor` accepts the mappings, but `check` intentionally remains unsuccessful
until source and secret checks replace `configure-enforcement` in the generated
`checks.toml`. Wire real tools for its listed rules and prove both compliant and
violating fixtures. Do not replace the barrier with `true` or broad waivers.
Native build/test success proves those commands only: it does not prove source
rules, PR size, one concern, coverage quality, or independent review.

Existing `.charter` files are never overwritten. To evaluate in an existing
project, add `--dir .charter-review` to init and check, then review the difference.
Commands always run from the invocation directory; `--dir` selects configuration.
The imported file is project-owned. Fleet can distribute pinned recipe sources
and this README, but source updates do not upgrade an existing import.

The integration test uses real npm/Node with a dependency-free disposable package.
It proves successful native scripts, a failing test, build-failure short-circuiting,
and the pending-enforcement barrier. It does not establish acceptance in your app.
