# Node/npm checks

This recipe wires `npm run build && npm test` into Charter. It requires Node/npm,
already-installed project dependencies, and `build` and `test` package scripts
that terminate and fail on errors. For TypeScript, the build script must run the
project's intended compiler checks. Nothing is installed. `init --detect` reads
the root `package.json` name only; it does not probe Node or npm.

From the project root, initialize Charter with its bundled recipe:

```sh
charter init --recipe node-npm
charter check
```

`doctor` accepts the mappings, but `check` intentionally remains unsuccessful
until source and secret checks replace `configure-enforcement` in the generated
`checks.toml`. Wire real tools for its listed rules and prove both compliant and
violating fixtures. Do not replace the barrier with `true` or broad waivers.
Native build/test success proves those commands only: it does not prove source
rules, PR size, one concern, coverage quality, or independent review.

Existing `.charter` files are never overwritten. To evaluate in an existing
project, add `--dir .charter-review` to both commands, then review the difference.
Commands run from the invocation directory; `--dir` selects configuration. The
generated file is project-owned. The recipe is embedded in the Charter binary,
so updating the binary does not upgrade an existing import automatically.

The integration test uses real npm/Node with a dependency-free disposable package.
It proves successful native scripts, a failing test, build-failure short-circuiting,
and the pending-enforcement barrier. It does not establish acceptance in your app.
