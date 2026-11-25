# Multi Instrument routed MIDI to two Mai Tai children

R394B used Studio Pro 8.1.1 build 113103 on macOS arm64 and the saved two-Mai-Tai
fixture. The selected scratch track's inspector showed All Inputs. The
trial registered one receive-only virtual Core MIDI source, verified the
12 outgoing messages through a loopback, enabled only the owned track's
monitor, and kept transport stopped and Main at zero.

| Input | Parent facade | Independently observed in both Mai Tai children |
| --- | --- | --- |
| Channel pressure | `control129` | `midi-cc.aftertouch` |
| CC1 | `control1` | `midi-cc.modWheel` |
| CC2 | `control2` | `midi-cc.breath` |
| CC11 | `control11` | `midi-cc.expression` |
| CC64 | `control64` | `midi-cc.sustain` |
| Pitch bend | `control128` | `midi-cc.pitchBend` |

Each row changed on the parent and both children in multiple bound reads.
The first five cases returned to exact zero. The parent sustain facade hit
127 while both child toggles hit 1. Center pitch bend returned 5000.30517578125
on all three providers, whose pre-event value was 5000. A separate read-bound
owner wrote the parent's original raw bits once. All 150 parent rows matched
afterward. Both child pitch-bend rows remained at the MIDI center encoding,
and the other 564 child rows matched. This also distinguishes a raw parent
write from routing the corresponding MIDI event to the children.

The first launch preflight stopped before registering the source. R394B
bound the intended scratch bytes and completed 297 requests in 84.021
seconds, then 57 in 5.453 seconds for the separate recovery. The host ran
197.996 seconds. The two owners, app and temporary sender exited normally.
The trial closed without saving. The original scratch bytes, the
18 managed files, and ten profile files including the exact original
MIDI configuration were checked after restoration. The six cases
verify virtual MIDI only.
