# Ampire Vibrato Speed in Studio Pro 8.1.1

R466B used Studio Pro 8.1.1 build 113103 on macOS arm64. The
440 Hz stereo control played only through an offline stem render with
Main muted at zero. The dry baseline and the three active stems each
contained 24 seconds of stereo PCM24 at 48 kHz. The trial used the stock
PAE Chorus 1 Mode switch at Pedalboard slot 5 and independently read the
bound `stomp5.vibratoMode` row before and after each switch. Every active
render boundary confirmed `Vibrato`.

The typed global Speed changed from the stock value near 0.30 to 0.625
normalized and returned to the original Float64 bits. The matching
`stomp5.vibratoRate` control followed the same values. The exact 88 peer
rows stayed fixed at every Vibrato before and after boundary, including
Depth and Chorus Intensity.

| Speed stage | Right sideband offset, 8–16 s | Right sideband offset, 16–24 s |
| --- | ---: | ---: |
| Original A | 5.9821 Hz | 5.9821 Hz |
| Changed B | 9.7137 Hz | 9.7137 Hz |
| Returned A | 5.9866 Hz | 5.9866 Hz |

Post-run FFT and an independent sinusoidal projection agreed on the
right-channel change and return. The left-channel settled windows did
not consistently show qualifying sidebands. These offsets measure the
processed stems for this exact fixture. They do not establish a general
normalized Speed to hertz conversion.

The stock switch returned to `Chorus`, the pedal returned off, and all
89 non-meter rows matched the initial baseline. Both same-instance
component captures were 2,297 bytes, but their SHA-256 hashes differed.
The live test stopped on that check. Both capture tokens were released.
The separate offline analysis retained the exact four WAVs and qualifies
the measured right-channel behavior. Full opaque component restoration
remains unverified.

The live owner made 182 requests in 280.753 seconds. The scratch closed
without Save and Studio Pro exited normally. The trial verified
restoration of 18 managed files and 12 app profile files.
