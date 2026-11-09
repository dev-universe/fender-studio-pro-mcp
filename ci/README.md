# Run local checks

Run the root server and embedded-script checks from the repository root.

```sh
rustup component add rustfmt clippy
./scripts/check.sh
```

The script requires Cargo and Node.js. It fails if either is missing. It checks Rust formatting, runs Clippy with warnings as errors, runs Rust tests, builds the release binary, and runs the actual bridge and EditTask assets in Node.js VM contract tests. Those VM tests simulate host objects. They do not establish actual DAW behavior. Python supports the separate scratch-session live plugin verification script.

The optional native extension has separate crates. On the supported macOS arm64 host, run their Rust 1.88 checks too:

```sh
for native_crate in parameter-api dsp-api editor-api addon; do
  cargo +1.88.0 fmt --manifest-path "native/$native_crate/Cargo.toml" -- --check
  cargo +1.88.0 clippy --manifest-path "native/$native_crate/Cargo.toml" --all-targets -- -D warnings
  cargo +1.88.0 test --manifest-path "native/$native_crate/Cargo.toml"
done
```

The first three crates use portable mocks; the add-on links macOS services. Normal Cargo checks need no vendor SDK. Optional exact-CCL header/compiler checks and package construction are described in the [native guide](../native/addon/README.md); do not substitute CCL 5.2 headers for the pinned 5.1.2 ABI. Building does not install or launch Studio Pro. The actual final run used 53 root Rust tests, 131 script tests, 60 native mocks and six native compile-fail documentation checks.

Node.js and Python are validation tools. The installed MCP server runs in Rust, and embedded JavaScript executes inside Studio Pro.

[rust.yml](rust.yml) is a workflow template. It lives outside `.github/workflows`, so GitHub Actions does not run it. The current OAuth credential lacks `workflow` authorization, and the connected integration returned HTTP 403 for workflow updates. No remote CI pass is claimed.

When an authorized credential can publish workflows, place the template in `.github/workflows/rust.yml`. The template runs Rust and JavaScript contract checks on macOS and Ubuntu and expects Cargo and Node.js on those runner images. Publishing it still requires workflow authorization; its existence is not a remote validation result.

Actual host results and remaining gaps belong in the [validation report](../docs/VALIDATION.md) and [plugin matrix](../docs/BUILTIN_PLUGINS.md). A compile or VM test pass does not verify Studio Pro integration on another operating system or host version.
