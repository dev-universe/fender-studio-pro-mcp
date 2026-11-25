# Ampire Wah on a fixed internal signal

R441C measured the exact Wah Wah role in Studio Pro 8.1.1 Build 113103 on macOS arm64 with the independently verified R359 candidate `5d0fea1eaf81fb24d3af9412182902ac7b58e45e`. The fixture fed a 440 Hz sine at −24 dB from Tone Generator into Ampire on the same owned stereo track. Its fader and Main were at zero, Main was muted, transport was stopped, and there was no hardware input or send.

The live catalog identified Wah Wah at index 15 and the editor showed it active in slot 2. Two independent DSP values, `controls.wah` and the bound `stomp2.pedal`, moved together from raw 0 to 6250 and returned to exact zero. The other 91 non-meter rows in the populated fixture matched.

| Wah window | Input peak range per channel | Output/Input median per channel |
| --- | ---: | ---: |
| Original zero | 630.386–630.683 | 4.279074 |
| Raw 6250 | 630.394–630.957 | 3.594312 |
| Returned zero | 630.397–630.956 | 4.275348 |

Each window has six stereo samples from separate read-only input and output peak taps. Input varied by less than 0.1% across the three windows. The changed ratio was about 16% below the neutral median in both channels. The returned-zero window tracked the first. The independent effect observed here is a two-channel peak difference on the fixed internal signal.

Tone Off and its ten rows, the 93 prepared Ampire rows, and the prepared component fingerprint returned. The first quiet check occurred while the peak meters were still decaying and stopped the primary driver. A new read-only owner bound the same document and confirmed both input peaks at zero and output peaks below `3.4e-9` in three reads. A subsequent owner sent the single remaining model reset. The editor showed the vacant slot and Tone Off. The initial component digest and 88 of 90 non-meter rows matched. The two residual readbacks were `tuner.diffCents` (5000 to 0) and the vacated slot’s `controls.active[stomp2]` alias (0 to 10000).

The primary run handled 123 requests in 62.373 seconds. The independent quiet confirmation used 16 requests and the final model recovery used 18. All four state capture tokens were released. The host ran 362.268 seconds, exited normally without saving, and the protected original, scratch file, 18 managed files, and ten app profile files were verified.

The first R441 setup stopped on the catalog’s braced class ID representation. R441B reached Wah discovery, where its candidate lacked the already-shipped exact Float list-bound fix. Neither preliminary setup enabled Tone or attempted a Wah write.
