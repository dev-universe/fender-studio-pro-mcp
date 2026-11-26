# Six routed SampleOne MIDI controls in Studio Pro 8.1.1

A receive-only virtual Core MIDI input changed six independently read DSP parameters on the exact SampleOne instance with a sample loaded. After release and two exact-bit corrections, all 235 typed rows and the loaded component SHA-256 matched the original captures.

## Fixture and observations

- macOS arm64, Studio Pro 8.1.1 Build 113103, diagnostic source `ea6390a318d7dd6df3627d7b7bb21a17a41148e6`, instrument class `C37BC9D1-6BD1-46A7-A60C-B13438666448`.
- A separate Core MIDI loopback verified all 12 packets. Studio Pro's external-device dialog registered the dedicated source with no send port. The track inspector showed that keyboard as the sole input to the SampleOne track.
- One generated mono PCM16 48 kHz WAV appeared in the SampleOne editor. The track had no events or record arming, transport stayed stopped, and Main stayed at zero. Monitoring was enabled for the six episodes and returned to off afterward.
- Each episode had 17 positive, exact-instance DSP reads. Two separate reads after each reset confirmed the released value and all five peer controls remained at baseline throughout.

| Message | DSP role | Active raw value | Released raw value |
| --- | --- | ---: | ---: |
| Channel pressure 96 | `aftertouch` | 7559.0550899505615 | 0 |
| CC 1 = 96 | `modWheel` | 7559.0550899505615 | 0 |
| CC 2 = 96 | `breath` | 7559.0550899505615 | 0 |
| CC 11 = 96 | `expression` | 7559.0550899505615 | 0 |
| CC 64 = 127 | `sustain` | 1 | 0 |
| Pitch bend 12288 | `pitchBend` | 7500.457763671875 | 5000.30517578125 |

The mod-wheel and pitch-bend release values differed from their original raw values of 314.96062874794006 and 5000. The MCP raw Float64-bit restores returned each exact value. Two full reads matched all 235 original rows, the loaded component hash matched, and all three capture tokens were released.

The verified owner completed 203 requests in 74.323 seconds. Studio Pro exited successfully after 129.630 seconds and the scratch archive hash was unchanged. All 18 managed installation files and 11 app settings files, including the original input profile, were restored. An earlier setup attempt stopped at bridge binding before any MIDI transmission. Physical input, audio output and saved MIDI events were outside this test.
