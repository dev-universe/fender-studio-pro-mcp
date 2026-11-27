# Contribute a change

Open a focused issue or pull request for a bug fix or feature. For a security
vulnerability, follow [SECURITY.md](SECURITY.md) and keep the report private.

Use a branch and pull request title without `codex`. Describe the behavior you
changed, the checks you ran, and any results you could not verify.

Give concurrent changes separate worktrees and build directories. After merge,
preserve validation artifacts and close the worktree using the
[worktree cleanup procedure](docs/MCP_DEVELOPMENT.md#작업을-병합한-뒤-워크트리를-정리한다).

## Run the offline checks

Install Rust 1.88.0 with `rustfmt` and `clippy`, Node.js 24, and Python 3. From the
repository root, run the same server and bridge checks as CI.

```sh
RUSTUP_TOOLCHAIN=1.88.0 bash scripts/check.sh
```

The script checks Rust formatting, Clippy, tests, a release build, JavaScript
tests, and Python tests. Local runs show free space on the checkout volume before
the build. The script needs no DAW or SDK.

For native changes, use a macOS arm64 machine. Run the library tests and build
the package without installing or loading it.

```sh
(
	export CARGO_TARGET_DIR="$PWD/target/native-addon"
	for crate in parameter-api editor-api node-api dsp-api preset-api; do
		cargo +1.88.0 fmt --manifest-path "native/$crate/Cargo.toml" -- --check
		cargo +1.88.0 test --manifest-path "native/$crate/Cargo.toml"
	done
	cargo +1.88.0 fmt --manifest-path native/addon/Cargo.toml -- --check
	cargo +1.88.0 test --locked --release --manifest-path native/addon/Cargo.toml -- --test-threads=1
	sh native/build.sh
)
```

The add-on tests run serially with optimization because they share mock host
state and exercise the native request deadline with large synthetic capsules.
Native builds currently report an existing `dead_code` warning for
`request::external_menu::observe`.

The `Core offline checks` and `macOS arm64 native checks` CI jobs run on pull
requests and pushes to `main`. Native tests use mocks. CI does not prove that a
command works in the DAW or with physical hardware. See
[native integration](native/README.md) for ABI and SDK validation requirements.

## Preserve user data and access guards

Keep the MCP server in native Rust. JavaScript belongs in the embedded DAW
bridge. Preserve request, session, document, instance, and cross-process access
guards. Do not expose arbitrary evaluation, shell execution, or unchecked
scripts.

Run any separately authorized live checks in a named scratch session. Preserve
user songs and settings. Report accepted requests, verified effects, and unknown
outcomes separately.

Keep songs, audio, raw app logs, private paths, credentials, and proprietary app
resources out of commits and public reports. Use small synthetic fixtures for
reproductions. Preserve MIT notices on copied bridge and EditTask assets.
