# Pedalboard moved-to-vacant slot persistence

R439 exercised the public `pedalboard_slot_swap` in Studio Pro 8.1.1 Build 113103 on macOS arm64, using the exact R343 candidate `d894e629f57c37d8ce3d070128cd783c69a05cb0`. The independent test song had Compressor in slot 1, slot 2 vacant, and a separate Phase Meter in the next insert. Main stayed at zero with transport stopped.

- One accepted public request moved Compressor to slot 2 and left slot 1 vacant. The editor and independent captures agreed on both positions and the eight slot fingerprints. The temporary active alias on the vacated slot still reported raw 10000 in that process.
- The moved layout was saved once, without an inverse swap. Independent inspection of the saved component matched the empty class in slot 1, the complete Compressor payload in slot 2 including Gain, Mode, activation and Peak, and the payloads at all other positions.
- The unchanged saved file opened in an independent, write-protected Studio Pro process. The editor again showed slot 1 vacant and Compressor in slot 2. The vacant slot’s active alias now reported raw zero. All 33 typed DSP rows matched after accounting for that one alias, and the component digest matched across processes. The Phase Meter, mixer, track, and stopped timeline matched the guarded fixture.
- One request to that same public tool with the previous document token was refused before dispatch. The first and second independent reads in the new process matched. The app’s closing Save prompt was declined.

The first owner used 88 requests in 59.830 seconds and the second used 45 requests in 19.787 seconds. The hosts ran for 116.408 and 59.178 seconds. Both owners and apps exited normally. The protected source, once-saved file, 18 managed runtime files, and ten app profile files were independently verified at closure.
