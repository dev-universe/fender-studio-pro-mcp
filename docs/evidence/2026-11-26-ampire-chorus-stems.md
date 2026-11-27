# Ampire Chorus Speed channel stems in Studio Pro 8.1.1

R452C ran in Studio Pro 8.1.1 build 113103 on macOS arm64. One synthetic
440 Hz input was imported once to an isolated stereo Audio track and
read back as a single 0–24 second event. Main was muted and at zero.
The stock Channels stem exporter wrote one 48 kHz stereo PCM24 WAV at
each stage, with 1,152,000 frames per file. Main was excluded from the
export. Transport stayed stopped before and after each export.

| Stage | Peak (dBFS) | RMS (dBFS) |
| --- | ---: | ---: |
| Dry before insertion | −24.000 | −27.010 |
| Active Chorus, Speed A | −10.150 | −16.181 |
| Active Chorus, Speed B | −10.145 | −16.174 |
| Active Chorus, Speed A restored | −10.142 | −16.172 |

The dry file met the predeclared calibration. Left/right gains were
1.00000000747/1.00000000760, RMS residuals were 7.280/7.259 PCM LSB,
and maximum residuals were 36/34 LSB. Each active stage held the same
Ampire instance and all other stable DSP rows. Only the bounded
activation, speed change, speed restoration and deactivation were
dispatched.

The predefined speed test stopped at the first A spectrum because the
left channel had no symmetric pair above −80 dBc between 0.375 and
12 Hz around the 440 Hz carrier in the 8–16 and 16–24 second windows.
Reading all the retained WAVs with the same analyzer found the best
left pairs at or below −159.216 dBc in these windows. The strongest
right pairs were spaced 3 Hz from the carrier at every stage with the
same reported levels within each respective window. A repeatable
Speed-dependent output frequency was not verified. The observed
levels describe the whole Ampire configuration relative to the dry
track and do not isolate an individual Chorus stage.

After deactivation, all 89 stable DSP rows and the exact 2,297-byte
component matched the baseline. The read-only indicator remained at
5362.783074378967 from an initial zero. Both component capture tokens
were released. The owner made 148 requests in 218.501 seconds. Studio
Pro exited normally after 254.312 seconds and the test song was
discarded without saving. The 18 managed files and 11 app profile
files were restored and verified. WAVs, raw captures, and scratch
files are kept out of the repository.
