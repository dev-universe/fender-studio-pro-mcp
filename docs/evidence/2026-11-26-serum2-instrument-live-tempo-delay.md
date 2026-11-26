# Serum 2 instrument delay follows Studio Pro tempo

R550 verified the Xfer-signed arm64 Serum 2 2.1.5 VST3
instrument in the local Studio Pro 8.1.1 Build 113103. The
public `serum2_load` tool connected it to a scratch Music track.
An exact diagnostic insert observed samples after the instrument.
The song contained one short C4 note and Main was at zero.

`serum2_preset_apply` accepted the empty rack, synced eighth-note
Delay and fixed 250 ms Delay on the same native instrument.
Each request pinned the current document, exact class and
instance UUID, and processor-preset SHA-256. Independent
inspection confirmed the complete 2,623-row DSP catalog on
that instance after each application.

| State | Delay from dry at 90 BPM | Delay from dry at 147 BPM |
| --- | ---: | ---: |
| Dry | 0 samples | 0 samples |
| Tempo-synced eighth note | 15,998 samples | 9,795 samples |
| Fixed 250 ms | 11,950 samples | 11,951 samples |

Each onset is the first sample above 0.0001 absolute amplitude,
relative to the dry reference at the same tempo. The sampler
reported 48 kHz and 512 frames per playing block in every
window. The observed delays agreed with the signed SDK renders
within two samples. They also matched the separately observed
Serum 2 FX onset differences in Studio Pro.
[SDK verification](https://github.com/dev-universe/fender-studio-pro-mcp/pull/322) ·
[Serum 2 FX verification](https://github.com/dev-universe/fender-studio-pro-mcp/pull/323)

R550 used 108 requests in 13.732 seconds, with six Play/Stop
cycles and 2,381 observer process calls. The app exited
normally after 48.258 seconds and the song closed without
Save. Both temporary bundles were moved out of the scan
path and all 18 managed files, ten profile files and the
scratch bytes were independently verified after restoration.
