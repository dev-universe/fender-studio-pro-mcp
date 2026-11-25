# Mai Tai MIDI reception in Studio Pro 8.1.1

R336D ran on macOS arm64 with Studio Pro 8.1.1 build 113103. An owned scratch
loaded the exact Mai Tai synth class `B625F134-4485-4A50-A3C8-C9CF0C5495E1`.
The selected track was unarmed and monitored from one temporary virtual Core
MIDI source. There were no arranged events, the transport stayed stopped,
and Main stayed at zero. A separate loopback first checked the twelve
message payloads. The existing inactive Mojito track was retained.

| MIDI 1.0 input on channel 1 | Mai Tai DSP observation | Active samples |
| --- | --- | --- |
| Channel pressure 96 → 0 | `midi-cc.aftertouch` 0 → 7559.0550899505615 → 0 | 17 |
| CC 1 modulation 96 → 0 | `midi-cc.modWheel` 0 → 7559.0550899505615 → 0 | 17 |
| CC 2 breath 96 → 0 | `midi-cc.breath` 0 → 7559.0550899505615 → 0 | 16 |
| CC 11 expression 96 → 0 | `midi-cc.expression` 0 → 7559.0550899505615 → 0 | 17 |
| CC 64 sustain 127 → 0 | `midi-cc.sustain` Off → On → Off | 16 |
| Pitch bend 12288 → 8192 | `midi-cc.pitchBend` 5000 → 7500.457763671875 → 5000.30517578125 | 17 |

Each episode had two fresh release reads. The other five roles retained
their own baselines. Changing the track back to its original input also
restored the original monitor state and mixer. The exact MIDI center still
left a pitch value of 5000.30517578125. One bound DSP write restored the
original `5000` bits, and two independent complete reads matched all 283
writable values. This is the same center behavior observed in Mojito.

The successful run used 169 requests and 95.540 seconds in the single owner.
The host was open for 128.867 seconds. Three preparation attempts stopped
before sending MIDI. Their guards exposed the braced catalog class ID, the
Main channel's absent record-arm field, and the Main row index moving after
track insertion. The fixture checks were corrected with those observations.
The scratch closed without saving, the original bytes matched, and all 18
managed files and 11 changed profile files were restored.

The observations concern parameter delivery and restoration. They do not
measure physical MIDI input, rendered audio, or the separate Character editor.
