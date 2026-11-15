# Java with Maven

This recipe wires `mvn test` into Charter. It requires Maven, a root `pom.xml`,
and dependencies available through the project's normal workflow. It does not
install Maven, resolve dependencies, or configure project-specific profiles.

From the Maven project root:

```sh
charter init --recipe java-maven
charter check
```

`init --detect` also selects this recipe from a root `pom.xml`. Detection does
not inspect the POM or probe Maven; review the imported command for your project.

`check` remains unsuccessful until real source and secret checks replace
`configure-enforcement` for its listed rules. Prove compliant and violating
fixtures for each tool; do not substitute `true` or broad waivers. Passing
Maven tests do not prove source rules, PR size, coverage quality, one concern,
or independent review.

Existing configuration is never overwritten. To evaluate in an existing
project, add `--dir .charter-review` to both commands, then review the difference.
Commands run from the invocation directory. Generated files are project-owned.
The recipe is embedded in the Charter binary, so updating the binary does not
upgrade an existing import automatically.
