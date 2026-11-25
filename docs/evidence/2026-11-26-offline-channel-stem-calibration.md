# Offline stereo channel stem calibration

R452B used Studio Pro 8.1.1 build 113103 on macOS arm64. A synthetic 440 Hz
stereo PCM24 reference at 48 kHz and approximately −24 dBFS peak was imported
through the stock file dialog. Independent MCP readback verified one unmuted
event from 0 to 24 seconds on the exact selected audio track. The track had
unity gain, centered pan, no hardware input, no inserts, no sends, and no
record arm or monitoring. Main was muted with volume at zero.

The stock Export Stems dialog exported only that stereo channel as a local
48 kHz, 24-bit WAV between the 0- and 24-second locators. Main was unchecked,
real-time processing off, and publishing disabled. The controlled folder
contained one 1,152,000-frame result. The original input and export retained
the same duration, sample rate, and nominal channel gain.

The initial acceptance required every sample to match within 1 LSB. It
stopped at the first nonzero source frame when one channel exceeded that
tolerance. A separate analysis of all sample words in the preserved export
fit both channel gains within `8 × 10⁻⁹` of unity. The zero-lag correlation
exceeded `0.999999999810` on each channel. Direct residuals
were at most 32 LSB left and 39 LSB right, with RMS 7.284 and 7.266 LSB.
The reason for the small nonzero render residual was outside this test.

The owner used 37 requests in 195.055 seconds. The app exited normally after
297.587 seconds and the scratch song was closed without saving. All 18
managed installation files and ten profile files were restored. This result
calibrates one dry stock channel export. It leaves effect processing and
hardware output as separate measurements.
