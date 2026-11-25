# Scope detector inhibition at the maximum Oneshot window

R431 compared the installed Studio Pro 8.1.1 Build 113103
`studioeffects` arm64 image against R420. The Mach-O UUID is
`CC2A5FBD-96A4-3231-9D30-DB782E03BE20` and the SHA-256 is
`14eb3a7bc2a37b7867fcf68fbf1acd6537f4a3bd3431ce383fe26317e7801dca`.
REA/Ghidra 12.1.4 supplied nine selected procedures. Craft Anything
0.6.1 supplied 1,396 instructions across the capture, detector and
parameter dispatcher. Their bytes matched the image and the LLVM
disassembly. [Earlier display path](2026-11-26-scope-capture-display.md)

| Path | Verified behavior in the installed image |
| --- | --- |
| `0x32b90–0x32c58`, Hold | Store the configured multiplier, invoke reset, and set the selected normal detector's inhibition to the rounded product of the capture window and Hold. An 80,000-sample window yields 80,000 samples at 100 percent and 160,000 at 200 percent. |
| `0x752c–0x755c`, Oneshot check | At entry, a count at least equal to the window plus an enabled hold mode and a latched trigger take the non-appending path. The hold path can still reach the display refresh branch. |
| `0x7564–0x75a0`, ordinary input | When that entry condition is unsatisfied, the normal detector can report a rising edge before the current block is appended. A detected edge sets the latch, updates the capture offsets and resets the accumulated count. The branch preserves the existing contents of the capture arrays. |
| `0xb6d8–0xb6f8`, detector | In the normal rising path an immediately previous sample below threshold and a current sample at or above threshold qualify after the inhibition. |
| `0x7aa0–0x7ab4`, display | Append updates the count and marks the full-width display refresh dirty. Display reading ranges are independent of the accumulated count. |
| `0x3259c`, Position | Position and Time Scale share an array reconfiguration and detector reset path. Trigger Channel, Slope, Level, Hold, Transients and some format/routing changes also reach reset paths. |

R423 used 100 percent Hold, zero Position, Oneshot and Trigger A
with a 41.3 Hz source. Its full-width frames before Tone Off differed
and the later frame after verified Off contained a new flat middle
with visible signal at the ends. The traced code permits another edge
before the current block completes the window. A new re-anchored
prefix can then be drawn together with the preceding tail. At the
song's nominal 48 kHz and reported 41.3 Hz, 69 cycles are about
1.671 seconds, about 194 samples beyond the 80,000-sample window.
Two such intervals land near 3.341 seconds, within R423's Off
request and readback bracket of 3.322–3.509 seconds.

This is a code-supported timing hypothesis. R423 did not observe
the actual repeat trigger, block boundaries, reset invocation or
live engine sample rate. [R423 timed host frames](https://github.com/dev-universe/fender-studio-pro-mcp/pull/214)

A fresh matched trial can change only Hold before arming. On the
exact bound Scope, `plugin_parameter_format` should independently
show 100 percent for the current raw value and 200 percent for
both 2000.0 and the predicted canonical raw
2000.0000298023224. Write the latter once while Tone is off,
verify the raw bits and formatted 200 percent, then use the same
maximum time scale, 41.3 Hz input, separate On and Off checks and
fixed viewport as R423. Compare full-width frames before and after
verified Off. This tests a display consequence. The internal trigger
latch, sample rate, hidden buffer and cause of the earlier frames
remain separate evidence.
