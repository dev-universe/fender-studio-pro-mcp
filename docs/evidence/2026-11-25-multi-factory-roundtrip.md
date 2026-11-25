# Multi Instrument factory recall and restoration

R326 used the macOS arm64 candidate with the Multi Instrument controller fix (`23b8069`) in Studio Pro 8.1.1 build 113103. It used one owned scratch song and one MCP owner, with playback stopped, Main silent, and instrument record arm and monitoring off.

| Issued factory index | Editor | Independently read controller slots |
| --- | --- | --- |
| 47 | `Temp - 1 Mai Tai - Controls` with a single connected Mai Tai | One Mai Tai |
| 48 | `Temp - 2 Mai Tai - Controls` with the visible split to Mai Tai and Mai Tai 2 | Two Mai Tai slots |
| 47 | Original preset title and single connection | Original ordered slot list |

At each stage, two separate reads agreed on the 150 typed root parameters and controller slot identities. The slot device and graph slot aliases agreed with each other. The initial and restored 150 rows and ordered slot identities matched exactly. A stale document was refused before dispatch. After explicit release, both the catalog token and the state token were refused. A subsequent independent read matched the restored projection.

The captured alias state changed between the one- and two-instrument presets. The original and restored captures were both 10,404 bytes with different hashes. A second capture in the restored state also produced a different hash while the root parameters and controller slots remained stable. Full state equivalence, per-child DSP values and the native audio processing routes are still unverified.

The MCP trial took 109 requests and 103.255 seconds. The host exited normally after 138.894 seconds. The scratch was closed without saving. All 18 managed installation files and their permissions, ten application profile files and the pinned inputs were verified after restoration.
