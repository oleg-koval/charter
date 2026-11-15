# Go module checks

This recipe wires `go test ./...` into Charter. It requires Go, a `go.mod` in the
invocation directory, and dependencies available through the project's normal
workflow. The command compiles and tests matched packages using the project's
Go environment. Charter does not install Go, modify modules, or configure caches.

From the module root, using a reviewed local copy of this recipe:

```sh
charter init --checks-from /path/to/recipes/go-module/checks.toml
charter check
```

This does not automatically cover nested modules, other workspace members,
alternate build tags, or every target platform. Customize commands and component
boundaries for those cases. Passing tests do not prove source rules, PR size,
coverage quality, one concern, or independent review.

`check` intentionally fails until real source and secret checks replace
`configure-enforcement` for its listed rules. Prove compliant and violating
fixtures for each tool; do not substitute `true` or broad waivers. `doctor`
validates mappings, not the effectiveness of tests or source enforcement.

Existing configuration is never overwritten. To evaluate in an existing project,
add `--dir .charter-review` to init and check, then review the difference. Commands
run from the invocation directory. Imported files are project-owned; Fleet can
copy pinned recipe sources and README without silently upgrading existing imports.

Import and mapping validation always run without Go. Native acceptance is opt-in
so Charter's standard test suite does not require Go:

```sh
cargo test --test go_recipe -- --ignored
```

This test runs real Go against a dependency-free disposable module,
with workspace inheritance and downloads disabled in the fixture. It proves
passing and deliberately failing tests, plus the pending-enforcement barrier.
It does not establish acceptance in a consumer repository.
