# Serum 2 mapped scalar writes in Studio Pro

R456B exercised Serum 2 and Serum 2 FX with the official Xfer-signed 2.1.5
VST3 in demo mode, Studio Pro 8.1.1 build 113103 and macOS arm64. The
instrument was selected, FX occupied slot 0 of a separate audio track, and
both native instances were bound independently. Main was muted at zero
volume. There was no external input, Send, recording or monitoring. The
timeline was empty and the transport stayed stopped.

The connected VST3 SDK probe identified these three Studio Pro DSP names.
Each exact instance accepted one normalized write per row and one restore
using the original raw Float64 bits. The generic MCP setter was
`plugin_parameter_set` with `readback: true`.

| Reference controller | DSP parameter name | Raw values in each variant |
| --- | --- | --- |
| CC 1 | `vstp516000000` | `0 → 2500 → 0` |
| Channel pressure | `vstp500000128` | `0 → 3750 → 0` |
| Pitch bend | `vstp516000001` | `5000 → 6250 → 5000` |

Each independent full DSP snapshot showed only the expected row change.
All 2,623 rows of the peer provider matched while the target was changed.
Both complete providers matched after each restore. The measurement covers
direct scalar control. MIDI delivery and the audio response remain separate
tests.

The earlier R456 run stopped at the route check before any parameter write.
Studio Pro's Audio track displayed the Korean unassigned input name, while
the instrument return reported `null`. R456B checked each form against its
exact current channel. It used 767 requests in 105.095 seconds, including
12 parameter writes. Each scratch song closed without saving. The owners
and hosts exited normally, each temporary signed VST3 was retired and all
18 managed installation files and ten profile files were restored.
