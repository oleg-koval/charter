.PHONY: build run test lint fmt fmt-check rules length secrets check hooks clean

AST_GREP := npx --yes --package @ast-grep/cli@0.51.0 ast-grep

build:
	cargo build --release

run:
	cargo run -- $(ARGS)

test:
	cargo test --all-targets

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

rules:
	$(AST_GREP) scan

length:
	scripts/check-file-length.sh

secrets:
	gitleaks git --redact --no-banner

# Everything CI runs, in CI order.
check: fmt-check lint test rules length secrets

hooks:
	lefthook install

clean:
	cargo clean
