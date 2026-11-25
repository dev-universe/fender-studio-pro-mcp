# Presence script macros, routed MIDI, and exact recovery

R349B exercised Studio Pro 8.1.1 build 113103 on macOS arm64. A disposable
song contained one Presence AudioSynth instance and an owned SoundFont.
The program supplied a test script entered through the Presence Editor.
The Main level was zero, transport stopped, the arrangement empty, and
record arm off. The Inspector showed the receive-only virtual keyboard;
the exact source was separately confirmed in the device settings and
with a four-packet Core MIDI loopback.

| Check | Independent evidence |
| --- | --- |
| Activate 16 macros | After each MCP write, a separate typed read matched. Eight Boolean values were 1. The eight continuous values were 1250 to 10000 in steps of 1250. The script verified each index, name and normalized value. |
| Route a note while active | The track monitor was enabled only for a pitch-60 note-on and its note-off. The native script log reported `NOTE 60 DISCARDED 1`. |
| Release all 16 | Separate typed reads were zero. The script reported its aggregate release marker after observing all 16 zero callbacks. |
| Route the same note while neutral | The same pitch and velocity were sent once more with a paired release. The native `onVoiceStart` marker appeared. The track monitor returned to off. |
| Restore the original program | An exact-instance capsule load returned the original whole-component hash and all 241 typed rows on two independent reads. The GUI showed the original SoundFont and empty script template again. |

The other 225 rows matched through both macro phases. Mixer, timeline,
document, bridge session and instance were checked throughout. The preset
title still showed the host's dirty marker after the capsule load. The test
establishes note disposition and voice creation for this program. Audio
output, saved-song persistence and other scripts were outside the trial.

An initial trial stopped after 40 requests at a missing harness
acknowledgement rule, before any macro edit or MIDI note. Its host
exited normally after 198.821 seconds. The corrected test used 56
editor requests in 59.695 seconds and 245 macro/routing requests in
112.272 seconds. The host exited normally after 219.700 seconds.
Six issued state tokens were released. Both MCP owners and the virtual
source exited normally. The scratch file was unchanged, and all 18
managed files and 11 touched app-profile files were restored.
