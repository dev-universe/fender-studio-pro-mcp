# Rumble Native input and output taps under an internal tone

R390B used Studio Pro 8.1.1 build 113103 on macOS arm64. The empty,
isolated stereo track had no physical input or timeline events and ran
Tone Generator into Rumble Native. Its fader and Main were at zero.
Transport stopped throughout. The 440 Hz sine used the inspected
Tone Generator setting of −24 dB.

| Condition | Input and output peak-tap range, raw |
| --- | ---: |
| Tone off before the trial | 0 |
| Amp active at position 0 | 667.635–668.208 |
| Amp bypassed at position 0 | 667.606–668.333 |
| Amp active at position 1 | 667.500–668.342 |
| Amp active at position 3 | 667.598–668.274 |

Each condition was observed in five instance-verified reads. In every
sample, the corresponding input and output peak values matched exactly.
The active and bypassed ranges overlap. The measurements establish
signal at the taps and leave audible processing, waveform identity,
and the actual route through individual blocks unresolved. The Tuner
screen opened with the tone present and displayed no note label.

The first attempt stopped before enabling Tone Generator because a
named parameter reply carried no instance field. The retry bound the
instance through the complete DSP page on each measurement. In R390B,
after switching Tone Generator off, the peaks decayed to nonzero
values below 0.000001 within the initial read window. That triggered
the exact-zero gate. A second read-only owner on the same session,
document and instances confirmed all four peaks at zero and double
readback of all 49 original DSP rows. The 26 processor identities and
22 numeric settings matched the baseline.

The run used 102 requests in 40.241 seconds, then ten requests in
2.163 seconds for the separate settled read. The host ran 122.630
seconds. The scratch was closed without saving and its bytes matched,
both owners and the app exited normally, and 18 managed files and ten
profile files were restored.
