# Serum 2 demo instrument delay across a song reopen

R551 used Studio Pro 8.1.1 build 113103 on macOS arm64, the Xfer-signed
Serum 2 2.1.5 VST3 in demo mode, and the same short C4 fixture and
downstream timing observer as R550. Main was at zero. At 48 kHz the
observer recorded the first sample above 0.0001 relative to the transport
start for each playback window.

In the first process, `serum2_preset_apply` applied one synthetic
processor-only preset with a fully wet, tempo-synced eighth-note delay to
the exact instrument instance. The input had a 1,426-byte `Comp` chunk
with the schema 11 rack maps. The owner verified the response, native
class and instance, played at both tempos, stopped, and closed its
session. Studio Pro saved the scratch song once and exited normally.

A separately launched Studio Pro process reopened that saved song under
a file-write guard. The old document token was refused. A fresh owner
confirmed the same instrument class and object ID, the C4 note, and the
same downstream observer, then played at both tempos without applying
any preset.

| Tempo | Prior dry reference (R550) | Before save (R551) | After reopen (R551) |
| --- | ---: | ---: | ---: |
| 90 BPM | 8,003 | 24,001 | 8,003 |
| 147 BPM | 4,900 | 14,695 | 4,900 |

The song passed ZIP integrity verification and contained a VST3 preset
for the same instrument class. Its 352-byte `Comp` chunk and 354-byte
`Cont` chunk both decoded as schema 11 metadata with `demoVersion: true`.
The saved processor chunk contained no FX rack maps. This agrees with
the metadata-only demo preset exported separately using the Studio Pro
preset menu. The demo restored the instance and note and played at
the dry onset upon reopening. This trial covers the instrument in demo
mode. Serum 2 FX and licensed save behavior require separate tests.
[Preset menu trial](2026-11-25-serum2-vst3-preset-load.md)

The warm owner issued 50 requests in 6.853 seconds. The cold owner issued
35 in 5.209 seconds. There were two play and stop windows per process.
The host processes ran for 56.844 and 31.735 seconds and exited normally.
The saved song hash was identical before and after the cold check.
The two staged plug-in bundles were moved out of the scanner directory.
Normal installation was restored and verified for all 18 managed files
and ten diagnostic profile files.
