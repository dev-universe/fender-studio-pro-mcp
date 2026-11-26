# Chorder audition permit in Studio Pro 8.1.1

R388 ran on Studio Pro 8.1.1 build 113103 on macOS arm64. The exact
candidate bound one owned scratch song and one Chorder Note FX through a
single MCP owner. Main was at zero, transport was stopped, and the note
effect editor was closed. The scratch had no arrangement events, armed
tracks, or monitored inputs.

- An independent preflight read verified the Chorder class, slot, instance,
  908 DSP rows, and the neutral key. Two model reads agreed on all 128
  chords. The fixture's MIDI 60 chord contained 60, 63, 67, 70, and 74.
- `prepare` issued a bound permit. A wrong instance and wrong document
  were refused. Replacing the permit invalidated the first one. Each
  check left the key neutral.
- The one accepted `play` acknowledged the press, separately observed the
  held input, acknowledged the guarded release, and separately observed
  neutral. The call returned `applied_verified` for the local DSP key lane.
  `status` and a duplicate `play` returned the same historical receipt.
- A separate expired permit was refused after 15 seconds. Independent
  reads confirmed that the input and the live and scheduled output
  flags were neutral. All 908 DSP rows and the 128-chord model
  matched the original, including the 581-byte model capture.

The trial did not measure downstream events during the permit's hold.
Earlier direct-controller trials R375 and R376 separately observed
local output flags and an independent downstream Chorder. Automated
owner and bridge tests cover lost acknowledgements and the quarantine
rule. No acknowledgement was deliberately dropped in R388.

The MCP owner completed 99 requests in 24.151 seconds, including permit
expiration. The host ran for 55.310 seconds. Both exited normally, the
trial song remained byte-identical, and 18 managed files and nine profile
files were restored.
