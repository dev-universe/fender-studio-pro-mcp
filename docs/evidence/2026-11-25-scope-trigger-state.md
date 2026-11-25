# Scope trigger state in Studio Pro 8.1.1

Static analysis of the arm64 `studioeffects` executable shipped with Fender
Studio Pro 8.1.1 build 113103. Its Mach-O UUID is
`CC2A5FBD-96A4-3231-9D30-DB782E03BE20`. The SHA-256 of the thin arm64
slice is `14eb3a7bc2a37b7867fcf68fbf1acd6537f4a3bd3431ce383fe26317e7801dca`.
REA with Ghidra located the declarations and decompiled the relevant branches.
An independent ARM64 disassembly verified the string references and branch
instructions where Ghidra's defined-string cross references came back empty.
Craft Anything CLI 0.6.1 in Deep mode independently decoded the instructions
that clear the trigger flag and store the mode in the restart routine.

| Observation | arm64 address | What it establishes |
| --- | --- | --- |
| Scope parameter declaration | `0x3025c` | `trigOneshot` (`0x5354724f`) and `trigRetrig` (`0x53547252`) both declare a 0–1 value with initial value 0. |
| Parameter dispatch | `0x32524`, `0x32c90` | Oneshot stores the requested mode. Retrigger reads that mode. Both call the same internal restart routine with the mode. |
| Oneshot parameter read | `0x3317c` | The getter reads back the configured mode. Retrigger has no matching branch in this getter. |
| Internal capture processing | `0x752c`–`0x765c`, `0x7d1c` | The restart routine clears an internal trigger flag. The processing routine uses a separate sample counter and sets the flag at a trigger transition. Its hold path tests the counter against the capture-window limit together with the mode and trigger flag. |

The existing R68 observations showed capture and hold of an approximately
134-pixel waveform period, then replacement by an approximately 67-pixel
period on retrigger. Separate MCP reads covered the parameters and the mixer.
See the [limited waveform observation](2026-11-15-scope-primary-functional-effect-native55.json).
This binary inspection explains why the Oneshot parameter read and the
Retrigger receipt leave acquisition start and completion unobserved.

No live app was opened for this inspection. The current MCP surface has no
independent reader for the flag or counter, and the full Scope unit still
requires separate acquisition timing, hidden waveform restoration and
resolution of the inactive-C UI/native discrepancy. The addresses apply only
to the identified binary and are evidence for the observed behavior.
