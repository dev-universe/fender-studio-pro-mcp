# Ampire global Volume and Pitch stem comparison

Fender Studio Pro 8.1.1 build 113103 on macOS arm64 rendered seven
isolated Channel stems around one exact Black Panel instance. The
owned synthetic source was a 440 Hz sine at −24 dBFS, identical in
both channels. Main stayed muted with its fader at zero, and the
track had no input or Sends. All WAVs were 24 seconds at 48 kHz,
PCM24 stereo, 1,152,000 frames. The stock renderer ran offline
with only the Ampire track selected. The transport was stopped
outside the seven renderer calls.

| Stem condition | Display and raw setting | RMS delta from first baseline |
| --- | --- | ---: |
| Global Volume | Empty, 0 → 7500 | +0.0063 dB |
| Global Pitch | Empty, 0 → 7500 | +0.0118 dB |
| Black Panel Normal Volume | 5.00 → 10.00 | +0.0165 dB |
| Restored baseline after those three changes | Original 98 stable rows | +0.0207 dB |
| Ampire output level | 0.00 → −9.60 dB | −9.5757 dB |
| Returned output level | Again 0.00 dB | +0.0274 dB |

The per-channel PCM analysis measured the middle and last eight
seconds. The two alias residuals were −62.76 dB and −57.34 dB
relative to the first stem. The two unchanged-settings repeats
differed by −52.44 dB and −49.98 dB. The signed cycle count gave
440 Hz for all 28 window/channel combinations at 0.125 Hz
resolution. A separate WAV reader agreed on the RMS changes.
Within this source, model, route and tested values, the alias
differences stayed inside the observed repeat baseline variation.
The output level was the positive audio control. The amp-local
Normal Volume did not pass the 3 dB positive-control threshold.

Independent reads matched all 98 stable settings after each
restoration. Both nonzero alias states had the same component
fingerprint as the first baseline. The last output-control round
trip restored those settings but changed the component capture
from 2,401 to 2,441 bytes. The content of that difference and
project save/reopen behavior remain unverified.

The completed renders took 164 requests in 170.308 seconds and
57 in 64.907 seconds. Both owners and the host exited with status
zero, seven captures were released, and the host ran 333.755
seconds. The source, scratch, 18 managed files and 11 changed
profile files were checked on cleanup. Earlier attempts reached
the stock import and output folder dialogs and stopped before any
render. Their installations were restored. No session was saved.
