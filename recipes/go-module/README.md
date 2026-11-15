# Go module checks

This recipe wires `go test ./...` into Charter. It requires Go, a `go.mod` in the
invocation directory, and dependencies available through the project's normal
workflow. The command compiles and tests matched packages using the project's
Go environment. Charter does not install Go, modify modules, or configure caches.

From the module root, initialize Charter with its bundled recipe:

```sh
charter init --recipe go-module
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
add `--dir .charter-review` to both commands, then review the difference. Commands
run from the invocation directory. Generated files are project-owned. Recipes are
embedded in the Charter binary, so updating the binary does not upgrade an existing
import automatically.

Import and mapping validation always run without Go. Native acceptance is opt-in
so Charter's standard test suite does not require Go:

```sh
cargo test --test go_recipe -- --ignored
```

This test runs real Go against a dependency-free disposable module,
with workspace inheritance and downloads disabled in the fixture. It proves
passing and deliberately failing tests, plus the pending-enforcement barrier.
It does not establish acceptance in a consumer repository.
