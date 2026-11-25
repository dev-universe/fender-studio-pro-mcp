# Multi Instrument reverse factory recall

R332 used an independent owned scratch on Studio Pro 8.1.1 build 113103 for macOS arm64. It applied factory index 48 as the baseline, index 47 as the target, and 48 again as the restoration. The same Multi Instrument parent remained bound. Playback was stopped, Main was silent, and the selected Music track had record arm and monitoring off.

Both readbacks at each stage and the editor showed two, one, then two Mai Tai slots. The baseline and restored readbacks agreed across all 150 root DSP rows, 283 typed DSP rows per child and the ordered controller slots. A stale document and mismatched parent, slot and child identities were refused, as were released factory and state tokens. The captured state bytes varied between repeated captures with stable DSP and slot observations.

The one-child target was saved once before returning to the two-child baseline. Its saved component contained three event nodes and two explicit successor links from one full-range key zone through a zero-transpose transformer to `Inst01`. The factory projection had the same ordered edges, node IDs and slot ID. Separate live controller reports exposed those same IDs. The final return to preset 48 was discarded on close. The single-child saved file remained byte-identical to the one verified save.

The MCP trial used 236 requests in 149.303 seconds, and the app exited normally after 229.084 seconds. All 18 managed files and permissions, ten profile files and the protected inputs were verified after restoration.

R330 completed the other direction using 47 as baseline and 48 as target. R331 verified the two-branch saved component and post-restart readback. The two factory cases establish restoration for these exact presets and scoped observed state. Native event delivery under input and broader Multi Instrument editing still need separate verification.
