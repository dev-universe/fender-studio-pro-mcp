# macOS arm64 installer lifecycle

The macOS 26.7.1 arm64 run used two isolated Studio Pro profiles and MCP
data directories. The CLI and native package were built with Rust 1.88.0
from both `ebdad4318df680a49365030c885dd0e3ab5879f7` and
`d743ca67a4974fc2dec379a1c554f80ab6bb3cd0`. All CLI calls explicitly set
`--profile` and `--data-dir`. The test denied writes to the vendor app and
the active installation paths. Studio Pro remained outside the test.

| Scenario | Result |
| --- | --- |
| Fresh installation | `install` and `native-install` installed 12 bridge and EditTask files and four native files. File digests agreed with the embedded source and native build manifest. The EditTask archive members agreed with the source and the generated mailbox settings. `doctor` verified both installations. |
| Clean uninstall | `uninstall` and `native-uninstall` removed all 16 managed files and both manifests. |
| Upgrade | Installing `d743ca67` over `ebdad43` changed three JavaScript files and the native executable. Each reported backup matched the previous file digest. The result, including both manifests, matched a fresh `d743ca67` install in the same profile. |
| Restore the previous build | Reinstalling the `ebdad43` CLI assets and native package restored the same 16 file digests, permissions and two manifests. It backed up the four replaced files. A repeat installation reported zero backups and reproduced the restored state. |
| Protect local edits | An edited bridge JavaScript file and edited native XML file caused both installers to refuse the update. `--force` backed up each modified file before replacement. After reapplying the edits, the uninstallers removed 14 managed files and preserved the two edited files and their manifests. |

The two isolated profiles retained eight synthetic settings and unmanaged
files with their original contents and modes. The 18 active MCP files had
identical digests and modes before and after the run. The 28 CLI calls and
assertions took 1.532 seconds after both builds completed.

The tested package is the local output of `cargo +1.88.0 build --locked
--release` and `sh native/build.sh`. The GitHub repository had zero Release
and Actions artifacts at the time of testing. The macOS CI job built and
validated the native package locally on its runner. Download, checksum
verification and installation from a published artifact remain open.
