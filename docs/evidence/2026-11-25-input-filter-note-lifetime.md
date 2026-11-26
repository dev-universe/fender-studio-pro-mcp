# Input Filter pitch boundaries and note release in Studio Pro 8.1.1

R384 used Studio Pro 8.1.1 build 113103 on macOS arm64, with Input Filter
and Chorder as the first and second Note FX on an owned instrument track.
The exact Input Filter class was `A7E3529F-6316-4E52-9A07-31739B027C88`.
An isolated receive-only virtual CoreMIDI source fed the track through All
Inputs. A loopback check observed every note-on and note-off message. The
track was monitored only during the bounded test, with Main at zero and
transport stopped.

The bound Input Filter contained the factory key bounds 48 and 72. Each
tested note appeared in Input Filter's live flags, while the separate
Chorder independently observed the following inputs during the hold.

| MIDI note | Downstream input |
| ---: | --- |
| 0 | Absent |
| 47 | Absent |
| 48 | MIDI 48, six positive reads |
| 60 | MIDI 60, six positive reads |
| 72 | MIDI 72, six positive reads |
| 73 | Absent |
| 127 | Absent |

Every note-off was followed by two neutral reads on both effects. Input
Filter's GUI and scheduled flags stayed neutral during these routed notes.
In the separate R383 test, writing the DSP GUI lanes for the same seven
pitches with monitoring off produced the matching Input Filter local
held flags and release. The downstream instance registered no held inputs
in that test. The two tests used different input paths and monitor states.

R384 restored the monitor and all 1,296 DSP rows across both effects. The
owner sent 399 requests in 50.039 seconds. The virtual source and host
closed cleanly, with the host running for 80.733 seconds. The scratch
bytes, 18 managed installation files and 10 app settings files, including
the temporary MIDI device configuration, matched their originals. These
observations cover the sampled MIDI boundaries and the software route.
