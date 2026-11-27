# Ampire retained microphone basis after exact linked recovery

R429 used Studio Pro 8.1.1 Build 113103 on macOS arm64. A fresh
owned scratch used the exact stock MCM 800 / 4x12 M65 Ampire
instance, with stopped transport and Main muted to zero.

The six R427 DSP writes matched each intermediate vector again.
The final linked state matched all 90 original typed rows and
the original 2,297-byte component. A separate observation gate
then verified fresh matching reads and authorized one Link Off on
the same exact instance. Two independent reads found the
following differences from the initial factory Link Off in R351.

| Mic | Initial factory unlink Float32 | Unlink after exact linked recovery | Difference |
| --- | --- | --- | ---: |
| A | `3F4BB3E9` | `3F4BB3E8` | −1 ULP |
| B | `3DD03F95` | `3DD03F94` | −1 ULP |
| C | `3DD2212A` | `3DD2212A` | exact |

Only Link and A/B changed from the immediately preceding linked
baseline. C, the other 86 typed rows, track, insert and mixer
remained exact. This matches the R425 prediction for the retained
basis. The exact linked and serialized values can coexist with
different later unlink behavior in the same app session. The
test did not qualify the basis after reopening the stored
recovered state. [R425 model](https://github.com/dev-universe/fender-studio-pro-mcp/pull/215) · [R427 exact linked recovery](https://github.com/dev-universe/fender-studio-pro-mcp/pull/216)

The owner made 168 requests in 68.357 seconds and released six
capture tokens. It performed no relink, Undo, Save, or state load
after the unlink. Both processes exited normally after 116.192
app seconds. The original and scratch bytes, 18 managed files,
and ten restored app profile files were checked.
