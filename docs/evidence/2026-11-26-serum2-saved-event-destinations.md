# Saved Studio Pro event destinations for the Serum 2 instrument

R507–R509 used the Xfer-signed Serum 2 2.1.5 instrument VST3 and
Studio Pro 8.1.1 build 113103 on macOS arm64. Each private song
contained one empty Music track and one exact Serum instance. Main was
muted at zero. Monitoring, recording and playback were off.

| Trial | Route at first native Save | Saved destination port |
| --- | --- | --- |
| R507 | Event Input 1 | `MusicInput00` |
| R508 | Event Input 2 | `MusicInput01` |
| R509 | Event Input 16 | `MusicInput15` |

A separate archive reader matched the Music track channel's
destination owner UUID to the saved instrument device UUID and the
live native instance UUID. Each of the alternate-port trials made
73 requests. The second-input trial took 48.488 seconds and the
sixteenth-input trial took 66.129 seconds. In each alternate trial,
all 2,623 typed DSP rows, the exact instance and the track and mixer
readbacks matched before and after Save.

In R507, changing the route with the direct channel method after the
first Save left the Save command disabled and the archive hash unchanged.
R508 and R509 assigned their respective ports before the first Save.
The route changes used the same guarded stock channel call qualified
by independent [live CC 74 delivery](https://github.com/dev-universe/fender-studio-pro-mcp/pull/286).

The file reader reports these two additional tokens for the exact
Serum 2 instrument class and preserves the existing first-port behavior
for instruments and inserts. CLI and stdio MCP checks against the
Studio Pro archives compared all three Serum 2 ports and the previously
verified first-port Serum 2 FX insert. The reads left all four files
byte-identical and completed in 1.715 seconds. Other alternate ports and alternate FX routes remain
outside the verified saved range.

The host and owner exited normally after each trial. The temporary
signed bundle was retired, all 18 managed files and 10 profile files
per trial were restored and verified, and no external MIDI was sent.
