# Exact recovery of a prepared linked Ampire cabinet mix

R352 used Studio Pro 8.1.1 build 113103 on macOS arm64. One exact MCM 800
and 4x12 M65 effect was added to a muted empty stereo track with silent
Main and stopped transport. The native editor showed the exact cabinet
and all three microphone faders.

| Stage | Link | Mic A raw | Mic B raw | Mic C raw |
| --- | ---: | ---: | ---: | ---: |
| Prepared baseline | 1 | 2500 | 2500 | 5000 |
| One linked MCP write to Mic C | 1 | 3125 | 3125 | 3750 |
| Link disabled | 0 | 2500 | 2500 | 3750 |
| Exact Mic C write followed by relink | 1 | 2500 | 2500 | 5000 |

The editor showed 31%, 31%, 38% after the linked write. Disabling Link
then returned A and B from 3125 to 2500. The first owner stopped when its stricter unlink expectation
failed. A new owner bound to the same exact document, session and
instance and independently verified the complete observed unlinked
state. It wrote the original Mic C bits once and re-enabled Link.

Two complete independent reads of all 90 non-meter settings and the
complete serialized component fingerprint matched the prepared baseline.
The editor showed 25%, 25%, 50% and Link enabled. The mixer, arrangement
and insert chain were unchanged. This is an exact round trip for the
prepared fixture. Recovery of the different factory starting values
remains open as documented in the linked and unlinked precision trials.

REA/Ghidra and Craft Anything 0.6.1 Deep resolved the declaration routine
at `0x9e60` and the `AmpireCabinet` configuration path at `0x1bcf8` in
the vendor arm64 slice (UUID `E83B8167-CF19-3B5B-A5C6-E7EBD743AB31`).
The live reads establish the values in the table.

The two owners used 108 and 34 calls in 49.324 and 18.553 seconds. The
host exited normally after 201.852 seconds. No song was saved.
The original scratch bytes, 18 managed files and 10 touched app files
were verified after cleanup.
