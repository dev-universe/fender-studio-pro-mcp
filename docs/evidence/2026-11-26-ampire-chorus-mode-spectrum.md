# Ampire Chorus mode in the retained Speed stem comparison

R464 independently analyzed the retained
[R452C stereo stems](https://github.com/dev-universe/fender-studio-pro-mcp/pull/247).
All four renders and the synthetic source matched the retained hashes
and contained 1,152,000 frames of 48 kHz stereo PCM24. R464 made no live
DAW calls. The original two-channel Speed criterion remains unfulfilled.

Independent carrier subtraction and a complex FFT found seven late
right-channel pairs around 440 Hz. The first pair had approximately
1.5132 Hz spacing from the carrier. The strongest pair was the second
one at approximately 3.0264 Hz on either side. Those peak locations
were stable in Speeds A, B, and returned A at the analysis reporting
precision. The 8-second analysis windows have a 0.125 Hz Rayleigh
resolution. The estimates describe relative agreement between files.

The left carrier was approximately 9.9 dB above dry in all active
renders. It had no comparable late modulation pairs. The full 24-second
PCM comparison found a small gain drift common to both channels,
including between A and returned A. After fitting one gain and one
local derivative per channel, the residuals were 88.1 to 99.5 dB below
the respective comparison signal. Their source remains unresolved.

All 12 complete R452C DSP pages, 89 prior active phase pages, and eight
earlier alias pages reported the same pedal mode, Chorus. On the stock
PAE Chorus 1 editor, Mode, Chorus Intensity, Vibrato Depth, and Vibrato
Rate are distinct controls. The archived Speed writes and their alias
readbacks therefore leave the separate Vibrato-mode response open.
The next controlled comparison can verify the actual Mode selection
and repeat Speed A/B/A with the same Intensity and Depth in that mode.
