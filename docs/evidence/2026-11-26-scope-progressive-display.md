# Observe a Scope trace fill progressively at the maximum time scale

R423 ran on Studio Pro 8.1.1 Build 113103 for macOS arm64. The
owned scratch had a dedicated empty stereo track with no input
or events. One bound Tone Generator fed one exact Scope.
Transport was stopped, monitoring and recording were off, and
both the track and Main were at zero with Main muted.

The Scope was armed in Oneshot, rising Slope, source A Main Left,
and threshold 37.5%. Maximum `timeScale` raw 10000 displayed
`8000.0 smp`. The trigger section already read `80000 smp`
while the trace was flat and the oscillator was Off. The saved
song specifies 48 kHz. The live engine sample rate was unavailable.

The owner scheduled one Off-to-Stereo transition on a 41.3 Hz
sine at −7.50 dB. Time zero in the table is the scheduled command
target. The On request and independent read were bracketed by
approximately 0–134 ms. The Off request and read were bracketed
by 3,322–3,509 ms. An additional read confirmed Off by 3,576 ms.
The same editor and viewport were used for all screenshots.

| Frame capture request interval | Visible A trace |
| --- | --- |
| −256 to −173 ms | Flat across the canvas |
| 100 to 180 ms | Oscillation only at the left edge |
| 441 to 520 ms | Oscillation across about one quarter of the width |
| 811 to 906 ms | Oscillation across about half the width |
| 1,251 to 1,333 ms | Oscillation across about three quarters of the width |
| 1,901 to 1,977 ms | Oscillation across the complete width |
| 2,721 to 2,799 ms | Full-width trace with different detail on the left |
| 4,601 to 4,687 ms | Trace persisted after Off with a flat central region and oscillation at the right |

The sampled frames establish progressive visible filling and a
subsequent full-width trace. The displayed waveform changed
again after the verified Off, so stable holding of that complete
trace was unverified at this scale. The intervals refer to request
and screenshot boundaries. Direct internal trigger and completion
times and the hidden buffer remain unobserved.

Tone Off and all 41 typed settings were verified by independent
reads, and the Scope returned to a flat default display. The
mixer and route matched throughout. The owner used 84 requests
in 88.469 seconds. The app and owner exited normally, with the
app open for 133.421 seconds. The scratch closed without saving.
Original hashes, 18 managed files and 10 restored app profile
files were verified.
