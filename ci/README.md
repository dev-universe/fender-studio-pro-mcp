# Run local checks

Run the complete local development checks from the repository root.

```sh
rustup component add rustfmt clippy
./scripts/check.sh
```

The script requires Cargo and Node.js. It fails if either is missing. It checks Rust formatting, runs Clippy with warnings as errors, runs Rust tests, builds the release binary, and runs the actual bridge and EditTask assets in Node.js VM contract tests. Those VM tests simulate host objects. They do not establish actual DAW behavior. Python supports the separate scratch-session live plugin verification script.

Node.js and Python are validation tools. The installed MCP server runs in Rust, and embedded JavaScript executes inside Studio Pro.

[rust.yml](rust.yml) is a workflow template. It lives outside `.github/workflows`, so GitHub Actions does not run it. The current OAuth credential lacks `workflow` authorization, and the connected integration returned HTTP 403 for workflow updates. No remote CI pass is claimed.

When an authorized credential can publish workflows, place the template in `.github/workflows/rust.yml`. The current template contains Rust checks on macOS and Ubuntu. Add Node.js and the JavaScript contract command before treating the workflow as equivalent to `scripts/check.sh`.

Actual host results and remaining gaps belong in the [validation report](../docs/VALIDATION.md) and [plugin matrix](../docs/BUILTIN_PLUGINS.md). A compile or VM test pass does not verify Studio Pro integration on another operating system or host version.
