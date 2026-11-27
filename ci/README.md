# Checks and verification

The active [GitHub Actions workflow](../.github/workflows/ci.yml) runs on pull requests and pushes to `main`. Both jobs are required for merging.

| Required check | Runner | Scope |
| --- | --- | --- |
| Core offline checks | Ubuntu 24.04 | Rust 1.88 formatting, Clippy with warnings denied, root tests and release build, JavaScript contract tests, Python tests |
| macOS arm64 native checks | macOS 15 arm64, verified with `uname` | Root tests, formatting and tests for five native libraries, optimized serial add-on tests, native package/plist/XML/dSYM build |

The workflow uses read-only tokens and full commit SHA pins for GitHub-owned Actions. It does not install Studio Pro, launch the DAW, use signing credentials or publish packages. The repository requires approval for all external contributors' workflow runs. Current results belong to the [Actions page](https://github.com/dev-universe/fender-studio-pro-mcp/actions); a workflow file alone is not a passing run.

Run the local checks from the worktree assigned to your change.

```sh
rustup toolchain install 1.88.0 --profile minimal --component rustfmt --component clippy
RUSTUP_TOOLCHAIN=1.88.0 bash scripts/check.sh
```

The script requires Cargo, Node.js and Python 3. It checks Rust formatting, runs Clippy with warnings denied, runs Rust tests, builds the release binary, executes the actual bridge and EditTask assets in Node.js VM contract tests, and runs the offline Python suite. VM tests simulate host objects; they do not establish actual DAW behavior.

On macOS arm64, run the native tests and package build in the same worktree.
The [local native commands](../CONTRIBUTING.md#run-the-offline-checks)
reuse that worktree's `target/native-addon/` directory.

The add-on's bounded large-capsule test exceeded its real two-second budget in a loaded local debug run. The CI uses an optimized serial test build without raising that budget. Two test assertions compared addresses of distinct C string literals; they now compare the metadata text, allowing the optimized suite to pass all 448 tests. A pre-existing `observe` dead-code warning in native external-menu code remains documented; the root server's Clippy gate still denies warnings.

Normal native builds need no vendor SDK and do not modify the Studio Pro app bundle. Optional exact-CCL header/compiler checks are described in the [native guide](../native/addon/README.md). Do not substitute CCL 5.2 headers for the pinned 5.1.2 ABI. Node.js and Python are validation tools; the installed server runs in Rust and embedded JavaScript executes inside Studio Pro.

[rust.yml](rust.yml) is an older, inactive template outside `.github/workflows`; it is not the current CI. Earlier workflow-publication permission failures are historical.

Actual host results and remaining gaps belong in the [validation report](../docs/VALIDATION.md) and [plugin matrix](../docs/BUILTIN_PLUGINS.md). A compile, unit-test or VM pass does not verify Studio Pro integration on another operating system or host version.
