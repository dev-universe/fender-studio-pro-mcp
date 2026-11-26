# Live Studio Pro VST3 timing at two tempos

R547C ran in the local Studio Pro 8.1.1 Build 113103 on macOS
arm64. A scratch Music track hosted a newly built VST3 diagnostic
instrument and an FX insert. Both request the same `0x456` context
mask measured on the signed Serum 2 and Serum 2 FX 2.1.5 in the
[SDK trial](2026-11-26-serum2-vst3-process-context.md). The
diagnostics retained context values inside the VST3 audio
`process` callback and wrote the bounded sample when the app
closed. The song contained one MIDI part and Main was at zero.

| Window | Instrument blocks | FX blocks | Context |
| --- | ---: | ---: | --- |
| 90 BPM, cycle disabled | 158 | 158 | `0x23F02`, playing, cycle points 0 and 0 |
| 147 BPM, cycle enabled | 338 | 338 | `0x23F06`, playing, cycle points 0 and 4.9 quarter notes |

Every playing block was realtime, 512 frames, 48 kHz, in 4/4.
Both reported 90 and 147 BPM at the matching transport windows.
The 147 BPM window had one loop wrap. Project samples moved
from 95,744 to 256 and musical position reflected the same
wrap. Continuous samples advanced by 512 through the wrap.
The independent verifier compared consecutive block positions
against the tempo and sample rate in both windows.

The instrument received 2,038 process calls and the FX received
2,018. Both recorded zero null contexts and zero calls to
`getProcessContextRequirements` in the DAW. The independent SDK
runner queried the same method on both diagnostics and read
`0x456`. This measures host delivery for the diagnostic classes.
The direct contexts of the commercial Serum instances and
audible changes to tempo-synced presets remain unverified.

An initial component without an edit controller left a blank
track. Adding a controller initialized the instrument. A separate
read found that the actual mixer channel used the instrument
name, which allowed the FX to bind to the correct channel.
That run confirmed the FX windows, but stopped callbacks had
already filled the instrument's original capture buffer. R547C
kept playing blocks and changes in stopped state to retain both
variants' windows.

R547C verified two MCP Play/Stop cycles in 61 requests and
8.720 seconds. The app closed without Save after 41.919
seconds. The temporary probe was removed from the scan path.
All 18 managed installation files, ten changed Studio Pro
profile files, and the scratch song bytes were rehashed after
restoration. Raw callback samples stayed local.
