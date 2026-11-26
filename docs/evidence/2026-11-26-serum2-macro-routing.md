# Serum 2 Macro 1 to oscillator A level

R483 and R484 used the Xfer-signed Serum 2 2.1.5 VST3 on macOS arm64.
The instrument fixture used the existing schema-11 processor state with
oscillator A gain set to 0.16. Its only difference from the control fixture
was one modulation route from Macro 1 to oscillator A level. The route
used source `[25, 0]`, oscillator module 0, parameter 1
(`kParamVolume`), and amount 65. The format follows the
[Ableton MCP Serum study](https://github.com/dev-universe/ableton-mcp/blob/36a4e7a/docs/serum-control-reference.md).

R483 sent VST3 parameter ID 7000000 through `IParameterChanges` during
one held note at 48 kHz. Each condition rendered 96,256 stereo frames
and used the same four automation points. RMS was calculated inside
the settled portion of each window. Both channels matched to the
reported precision.

| Fixture | 0% | 50% | 100% | Back at 0% |
| --- | ---: | ---: | ---: | ---: |
| Route absent | 0.03268 | 0.03274 | 0.03268 | 0.03271 |
| Macro 1 to oscillator A level | 0.03268 | 0.10756 | 0.20428 | 0.03271 |

R484 used Studio Pro 8.1.1 build 113103. The MCP applied the routed
processor-only carrier to one exact instrument. The apply receipt
matched the instance and file SHA-256. A separate Rumble Native insert
reported the stereo input peaks. The same one-note C4 arrangement ran
three times with Main muted at zero, no external input, and monitoring
and recording off. The two exact Macro 1 writes used
`plugin_parameter_set` on `vstp7000000` with readback enabled. The first
set `normalized: 1` and the second restored the original raw Float64
zero. Each write had matched readback.

| Macro 1 | Eligible reads | Left raw median | Right raw median |
| --- | ---: | ---: | ---: |
| Initial 0% | 8 | 839.4112 | 839.4115 |
| 100% | 7 | 5247.8904 | 5247.8904 |
| Restored 0% | 7 | 838.6533 | 838.6534 |

Independent scans covered all 2,623 DSP rows at each step. Macro 1
was the sole DSP row that changed during the experiment, and all rows
matched the applied state after restoration. Each Stop was followed by
three consecutive quiet stereo observations. The exact carrier and
scratch song remained unchanged.

R484 took 117.528 seconds and 743 requests, including the three
Play/Stop pairs. Studio Pro exited normally after 136.924 seconds.
The scratch closed without saving. All 18 managed files and ten app
profile files were restored and the temporary signed scan bundle was
retired. The observations cover this instrument route and its internal
signal. Serum 2 FX modulation, hardware output, and editable preset
persistence remain separate tests.
