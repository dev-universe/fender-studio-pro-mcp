# Ampire linked cabinet microphone mix and capsule readback

R350 used Studio Pro 8.1.1 build 113103 on macOS arm64, with a single
Ampire insert set to MCM 800 and 4x12 M65 on an empty muted stereo track.
Main was silent, transport stopped, and record and monitor were off.
The exact effect, document and bridge session were verified.

With Mic Mix Link enabled, the MCP set only Mic B to normalized 0.25.
Independent reads showed changes in all three microphone rows. The link,
other typed settings, insert chain, mixer and timeline matched.

| Native role | Original raw value | After the Mic B write | After one capsule load |
| --- | ---: | ---: | ---: |
| Mic A | 7957.139611244202 | 6643.378138542175 | 7957.139015197754 |
| Mic B | 1016.8377310037613 | 2500 | 1016.8377310037613 |
| Mic C | 1026.0231792926788 | 856.6219359636307 | 1026.0231792926788 |
| Mic Mix Link | 1 | 1 | 1 |

The exact-instance capsule load was accepted. A fresh MCP owner made
two matching native reads of all 90 non-meter rows. Only Mic A differed
from the original typed values, by -0.0005960464477539062 raw. The
commented level remained at 80% on the typed display. The original
component capture was 2297 bytes, and two captures after restoration
matched each other at 2294 bytes with a different whole-component hash.
The trial stopped at that divergence. No Mic C, unlinked or compensating
write followed the restore attempt. Audio and physical output were not
measured.

The two owners used 48 and 10 requests in 4.638 and 1.832 seconds,
respectively. The app exited normally after 78.707 seconds. The song
was not saved, its bytes were unchanged, and all 18 managed and 10
touched app-profile files were restored.

[Fender Studio Pro Ampire controls](https://fenderstudiopromanual.fender.com/en/Content/Built-In_Effects_Topics/Distortion.htm)
