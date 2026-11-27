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

## Global Wah with the same model in two slots

R587 used Studio Pro 8.1.1 build 113103 on macOS arm64. It populated
slots 2 and 3 with the stock Wah Wah model in both selection orders.
The exact local pedal markers were 2500 and 7500. In both orders the
global read matched slot 2. A global write to 6250 changed only the
global row and slot 2 pedal. Slot 3 stayed at 7500. Independent typed
reads confirmed the match, the preserved peer and the restored markers.

Clearing both slots left the familiar vacant slot activation flags and
a detached global value with an empty display. After restoring the two
activation flags and clearing the observed global value, all 89 original
stable rows and the original component fingerprint matched. The result
covers these two slot positions with both models active. Audio for this
duplicate configuration was outside the test.

The first owner handled 117 requests in 11.358 seconds. A fresh owner
resumed from the observed residual state and handled 134 requests in
13.840 seconds. Three temporary captures were released. The host closed
without Save after 131.843 seconds. The scratch digest, 18 managed files,
and ten changed app-profile files were verified after restoration.

R588 disabled slot 2 while both Wah Wah models were still selected. The
global control continued to read and change slot 2. Re-enabling slot 2
restored the prepared pair. Removing that model retargeted the global
control to the surviving slot 3. In each case a write to raw 6250 changed
only the selected global and local rows, and the original marker returned.
After removing the two models, the empty-display global shadow at 7500
and the vacant activation flags were cleared. All 89 original stable rows
and the component fingerprint matched.

R588 handled 181 requests in 17.416 seconds and released two captures.
The scratch remained byte-identical with no Save, the host exited after
67.678 seconds, and the installation and ten changed profile files were
verified after restoration.
