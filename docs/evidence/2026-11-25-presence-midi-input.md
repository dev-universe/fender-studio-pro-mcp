# Presence SoundFont MIDI reception and release

R346C used Studio Pro 8.1.1 build 113103 on macOS arm64. A separate
Core MIDI loopback established all 12 bounded messages, in order.
The virtual source was registered before Studio Pro opened. A
temporary receive-only keyboard used that source and became the
default instrument input for the new owned scratch track. The track
inspector showed that keyboard by name. The Presence AudioSynth held
the owned Sine SoundFont with no events, stopped transport and silent
Main. Only the target's monitor was enabled, with record arm off.

| MIDI message | Exact Presence role | Active raw value | Release raw value |
| --- | --- | ---: | ---: |
| Channel pressure 96 | `midi-cc.aftertouch` | 7559.0550899505615 | 0 |
| CC1 96 | `midi-cc.modWheel` | 7559.0550899505615 | 0 |
| CC2 96 | `midi-cc.breath` | 7559.0550899505615 | 0 |
| CC11 96 | `midi-cc.expression` | 7559.0550899505615 | 0 |
| CC64 127 | `midi-cc.sustain` | 1 | 0 |
| Pitch bend 12288 | `midi-cc.pitchBend` | 7500.457763671875 | 5000.30517578125 |

Each role had 16 or 17 active reads and two independent matching
release reads. The other five roles stayed at their neutral values
during each episode. After disabling monitor, one exact-bit setter
returned pitch bend from the MIDI center to its initial raw 5000.
Two more full reads verified all 241 rows. The loaded component
fingerprint and the original scratch mixer also matched. Three capture
tokens were released.

The successful run used 203 requests in 97.768 seconds with the
app open for 148.746 seconds. Two preparation trials stopped before
MIDI dispatch. One could not select the track input through the window
interaction API. The other correctly refused the newly scoped profile
change until the trial validated that it contained only the exact
temporary keyboard. All scratch documents closed without saving and
retained their original bytes. The virtual source, temporary profile
changes and 18 managed MCP files were restored or removed as appropriate.
The observation covers the six input roles with a loaded sample.
