# Pedalboard nonadjacent exchange after Save and reopen

R442 exercised one public `pedalboard_slot_swap` in Studio Pro 8.1.1 Build 113103 on macOS arm64 using the clean R343 candidate `d894e629f57c37d8ce3d070128cd783c69a05cb0`. Main gain was zero, transport was stopped, and the next insert was the independently observed Phase Meter.

The guarded exchange swapped slot 1 Compressor with slot 3 Big Fuzz. The editor showed Big Fuzz, Reverb and Compressor in positions 1, 2 and 3. The middle Reverb retained its exact seven typed rows and serialized payload. The standalone Pedalboard’s divider role remained absent. Independent captures matched both moved payloads and all eight slot fingerprints.

The changed rack was saved once. The file held the two exchanged payloads, original Reverb, five vacant slots and unchanged input and output settings. A fresh write-protected Studio Pro process showed the same positions. The component digest and all 42 stable typed rows matched. The two read-only output peak values were recorded separately, with their row metadata checked. The Phase Meter, mixer and empty timeline matched the fixture. One previous-document request to the public tool was refused before dispatch, and fresh reads preserved the reopened state.

The two owners used 92 requests in 58.859 seconds and 45 requests in 15.985 seconds. The two hosts ran for 90.369 and 61.319 seconds. Both owners and hosts exited normally after the reopen session’s Save prompt was declined. The original and once-saved file hashes, 18 runtime files and ten profile files were verified at closure.
