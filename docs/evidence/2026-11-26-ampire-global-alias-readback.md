# Ampire global Volume and Pitch readback

R538 on Fender Studio Pro 8.1.1 build 113103, macOS arm64, used one
class-qualified Ampire instance in an owned scratch song. The Audio
track had no input, events, or Sends, and both the track and Main
were muted. Transport stayed stopped. The earlier
[read-only sweep](2026-11-25-ampire-volume-pitch.md) observed raw
zero and empty display text for both global rows.

With Black Panel selected, the independent, amp-local Normal Volume
control moved from 5.00 to 10.00 and returned to 5.00. It changed
no other stable row. The first owner stopped after global Volume
returned a verified readback at 7500, exposing a wrong assumption
in the fixture's original acceptance check. The second owner bound
the same native UUID and read the complete settings twice before
sending another edit.

| Global row | DSP tag | Raw sequence | Display |
| --- | ---: | --- | --- |
| `controls.volume` | 57347 | 0 → 7500 → 0 | Empty throughout |
| `controls.pitch` | 57349 | 0 → 7500 → 0 | Empty throughout |

At each nonzero value, the respective alias was the only changed
stable row. Both reversals restored all 98 stable rows. Complete
component serialization was byte-identical at the two nonzero and
two restored observations. Four read-only meters and the read-only
tuner sensor are outside the stable comparison. The first owner
issued 68 requests in 6.952 seconds, the second 46 in 5.126 seconds.
All six captures were released. The host and both owners exited
with status zero. The scratch file, 18 managed files and 10
changed profile files were verified against their original bytes.

The result establishes live raw readback and reversal for the two
rows in this exact Black Panel configuration. Audible processing
and saved-state persistence remain open. No playback, MIDI or Save
was involved.
