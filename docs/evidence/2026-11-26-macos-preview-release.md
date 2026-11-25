# macOS arm64 preview download and installer lifecycle

GitHub published `v0.1.0-preview.1` on November 26, 2026 at 02:52 KST.
The Git tag points to `1654cd10d85381b217f4275bd509af190c13ab34` from
main. The tree is identical to PR #208's exact tested head. Both its
Core offline and macOS arm64 required checks passed. The archive
includes the second stock Mai Tai command from that merge.
[Release assets](https://github.com/dev-universe/fender-studio-pro-mcp/releases/tag/v0.1.0-preview.1) · [Exact PR checks](https://github.com/dev-universe/fender-studio-pro-mcp/actions/runs/45666764150)

| Check | Observed result |
| --- | --- |
| Archive | 4,988,560 bytes, 9 regular files, macOS arm64 CLI and native bundle |
| Release checksum | `db04621a6c6c435cf05ccb65e070fc7ca259828ac31d44f57f76a66813691a7b` |
| GitHub draft and public assets | Both archives and sidecars matched locally byte-for-byte. The public pair downloaded over HTTPS with no GitHub authentication. |
| Privacy and package layout | No user home paths in either executable. Signed native bundle resource and both executable signatures passed strict checks. Archive native UUID matched the local dSYM. |
| Clean installation | Current installer and native bundle installed in a disposable profile and removed cleanly. |
| Update and rollback | Legacy archive at `49f7e762df2d` upgraded to the exact clean current state. The preview CLI restored all legacy managed bytes and removed the newer resource signature. The old CLI refused the newer manifest without changing it. |
| Removal | Restored files removed and all unmanaged sentinels preserved. |

The lifecycle check ran 21 CLI commands on the independently downloaded
archive. It used a disposable profile and data directory, with filesystem
writes to the actual Studio Pro installation denied. The lifecycle check
took 1.846 seconds.

The CLI and native resources have ad-hoc signatures. Developer ID and
notarization have no release evidence. The default Gatekeeper assessment
rejected the CLI on the verification Mac. The release is marked as a
developer prerelease.
