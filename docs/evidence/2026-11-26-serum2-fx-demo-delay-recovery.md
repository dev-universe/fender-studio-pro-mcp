# Serum 2 FX demo delay on reopen and explicit preset reapply

R552 used Studio Pro 8.1.1 build 113103 on macOS arm64 and Xfer-signed
Serum 2 FX 2.1.5 VST3 in demo mode. The instrument produced a controlled
stereo burst into Serum 2 FX at insert slot 0. The timing observer was
at slot 1 and Main was at zero. At 48 kHz the observer measured the first
sample above 0.0001 relative to the start of each playback window.

An owner applied one synthetic processor-only, fully wet eighth-note
delay preset to the exact FX instance with `serum2_preset_apply`. Studio
Pro played at both tempos, saved the scratch song once, and exited. A
new process reopened the song under a file-write guard. Its MCP owner
confirmed the same native FX class and instance UUID, note, and insert
order. The prior document token was refused.

| Tempo | Before save | After reopen | After explicit reapply |
| --- | ---: | ---: | ---: |
| 90 BPM | 18,403 | 2,405 | 18,403 |
| 147 BPM | 12,200 | 2,405 | 12,200 |

The first two cold playbacks used the restored song state. The owner
then stopped transport and applied the retained source preset once, with
the fresh document, exact slot and instance, and original SHA-256. The
last two cold playbacks reproduced both warm onsets exactly. The
`applied_unverified` receipt was followed by independent target
inspection and playback. This recovery uses the original local
processor-only preset and leaves the saved song unchanged.

The saved ZIP passed integrity verification. The FX preset in the ZIP
used the same VST3 class as the source. Its 356-byte processor and
359-byte controller chunks decoded as schema 11 with `demoVersion: true`
and no FX rack map. The input's 1,320-byte processor chunk contained
three rack maps. The observed dry reopen and recovery apply to this
demo configuration and test preset.

The warm owner issued 52 requests in 7.229 seconds. The cold owner issued
64 in 8.629 seconds for two dry and two recovered play windows. The
host processes exited normally after 52.793 and 41.541 seconds. The song
hash matched after the cold trial. Both staged plug-ins were removed
from the scanner directory, the 18 managed assets were rehashed against
the originals, and ten changed host profile files were restored.
