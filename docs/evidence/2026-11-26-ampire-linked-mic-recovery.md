# Exact linked Ampire microphone gain recovery with a final C write

R427 used Studio Pro 8.1.1 Build 113103 on macOS arm64. An exact
MCM 800 / 4x12 M65 Ampire insert was bound in an empty owned test
song. Transport was stopped, Main was muted at zero, and all three
microphones were enabled. The starting vector was the same as the
untouched factory state in R422.

The MCP performed six DSP writes. Each step required two complete
independent 90-row reads, exact Float64 and normalized Float32
comparisons, and unchanged mixer, track and insert binding before
the next write. All six observed vectors matched the R425 model.

| Stage | Link | A / B / C normalized Float32 bits |
| --- | --- | --- |
| Factory baseline | 1 | `3F4BB3E9 / 3DD03F95 / 3DD2212A` |
| Linked B set to raw 2500 | 1 | `3F2A120B / 3E800000 / 3DAF6FA9` |
| Unlinked | 0 | `3F4BB3E9 / 3E800000 / 3DD2212A` |
| A prepared | 0 | `3F4BB3E8 / 3E800000 / 3DD2212A` |
| B prepared | 0 | `3F4BB3E8 / 3DD03F92 / 3DD2212A` |
| Relinked | 1 | `3F4BB3E8 / 3DD03F94 / 3DD2212C` |
| Linked C set to its original raw value | 1 | `3F4BB3E9 / 3DD03F95 / 3DD2212A` |

The last write restored the exact original gains and all 90 typed
non-meter rows. Two independent component captures also agreed with
the original 2,297-byte fingerprint. Mixer, track, insert and timeline
were preserved. The fixture closed without saving, Undo or later
unlinking. This verifies the linked output vector and serialized
component for this factory configuration. A later unlink exposes a
separate basis that remains unverified. [R425 arithmetic](https://github.com/dev-universe/fender-studio-pro-mcp/pull/215)

The owner made 138 calls in 45.841 seconds and released all four
capture tokens. Both processes exited normally. Studio Pro was
open for 96.754 seconds. The original and owned scratch hashes
were unchanged, with 18 managed files and ten app profile files
verified after restoration.
