---
description: Build release binary, verify, and tag a new version
---

Perform a full release of lazycurl. The user may provide a version number as an argument (e.g., `/release 0.2.0`). If no version is provided, ask for one.

Steps:

1. **Verify clean working tree**: Run `git status` and ensure no uncommitted changes. If dirty, stop and ask.

2. **Update version in both Cargo.toml files**:
   - `crates/lazycurl-core/Cargo.toml` — update `version = "X.Y.Z"`
   - `crates/lazycurl/Cargo.toml` — update `version = "X.Y.Z"`

3. **Run full verification**:
   - `cargo fmt --all --check`
   - `cargo clippy --workspace -- -D warnings`
   - `cargo test --workspace`
   - If any step fails, stop and report the error. Do NOT proceed.

4. **Build release binary**:
   - `cargo build --release --workspace`
   - Report the binary location: `target/release/lazycurl` (or `lazycurl.exe` on Windows)

5. **Commit version bump**:
   - `git add crates/lazycurl-core/Cargo.toml crates/lazycurl/Cargo.toml Cargo.lock`
   - `git commit -m "release: v{VERSION}"`

6. **Tag the release**:
   - `git tag v{VERSION}`

7. **Report summary**:
   - Version: vX.Y.Z
   - Binary: target/release/lazycurl(.exe)
   - Tag: vX.Y.Z
   - Remind: `git push && git push --tags` to publish (do NOT push automatically)
