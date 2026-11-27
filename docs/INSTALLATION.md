# Installation and maintenance

Start with the [bridge quick start](../README.md#quick-start).

## Optional native add-on

Install the add-on to enumerate native plugin parameters and use supported state capture, presets, and deeper plugin inspection. It requires the tested Studio Pro 8.1.1 arm64 build, Python 3, and the macOS command-line tools for the build.

Quit Studio Pro and stop the MCP server or `hold` before installing. From the repository root, run the build with the Rust 1.88.0 toolchain installed.

```sh
sh native/build.sh
./target/release/fender-studio-pro-mcp native-install artifacts/native/fender-studio-pro-mcp.native.mac
```

Open Studio Pro and the scratch song, then start the MCP client. For native reads and writes, carry the returned `instance.object_uid` as `expected_instance`. Enumerate parameters again after changing models, pads, or plugin focus. [Native add-on guide](../native/addon/README.md)

## Troubleshooting

| Symptom | What to check |
| --- | --- |
| `controller_busy` | Stop the other MCP server, `hold` process, or live CLI command using the same profile. |
| MIDI source disconnected after a restart | Reselect **Receive From** while the server is running. |
| The bridge request times out on the Home screen | Open a scratch song and retry the connection check. |

## Update or uninstall

Quit Studio Pro and stop the server first. After updating the source, rebuild and run `install` again. If the native add-on changed, rebuild and reinstall it separately. The installers report backup paths for the files they replace.

To restore an earlier build, run its `install` command and pass its native package to `native-install`. Use the same Studio Pro profile and MCP data directory. Run `doctor` from the restored build to check the files.

If either the installed bundle or the package contains `CodeResources`, use a CLI that supports signed bundle resources for native install, restore and uninstall. A CLI built before that support may refuse to restore a legacy bundle from a signed installation. Use the newer CLI's `native-install` with the legacy package to preserve the signature resource in a backup.

To remove the installed assets, run `uninstall` for the bridge and `native-uninstall` for the add-on. Both preserve modified files. [macOS installer lifecycle verification](evidence/2026-11-25-macos-installer-lifecycle.md)

## Build a macOS arm64 archive

On Apple Silicon, run `sh scripts/package-macos-arm64.sh`. It writes a
versioned archive and a SHA-256 sidecar to `artifacts/macos-arm64`. You can
pass a different output directory as the first argument. To check a copied
archive, run `shasum -a 256 -c` with its `.sha256` file from that directory.
The archive contains the CLI, its native add-on, and a build manifest. The
archive's native bundle has an ad-hoc resource signature. Developer ID and
notarization are separate from this build.

The macOS arm64 developer preview includes the archive and SHA-256 sidecar.
Gatekeeper rejected its CLI on the verification Mac. [Preview downloads](https://github.com/dev-universe/fender-studio-pro-mcp/releases/tag/v0.1.0-preview.1) · [Installer verification](evidence/2026-11-26-macos-preview-release.md)
