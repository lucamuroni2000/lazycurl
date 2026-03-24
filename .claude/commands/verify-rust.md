---
description: Run full Rust verification suite (format, lint, test)
---

Run the following commands in sequence. Stop on first failure and report the error:

1. `cargo fmt --all --check` — verify formatting
2. `cargo clippy --workspace -- -D warnings` — lint with warnings as errors
3. `cargo test --workspace` — run all tests

If all pass, report: "All checks passed: formatting, linting, and tests."
If any fail, report which step failed and the error output.
