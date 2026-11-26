# Serum 2 FX virtual MIDI delivery in Studio Pro 8.1.1

R462B used the Apple-notarized Xfer Serum 2 FX 2.1.5 VST3 and Studio
Pro 8.1.1 build 113103 on macOS arm64. One exact Serum 2 FX was
bound at insert slot 0 of a stereo Audio track. The selected Music
track started at the independently bound Serum 2 instrument. The
stock Out menu then selected Serum 2 FX, and only the Music track's
monitor was enabled. The exact temporary Core MIDI input was matched
by name and ID. Main was muted at zero. No Audio track input, notes,
playback, recording, or Sends were active.

| First-channel message | MIDI active → neutral | FX raw DSP baseline → active → neutral |
| --- | ---: | ---: |
| CC 1 | 96 → 0 | 0 → 7559.0550899505615 → 0 |
| Channel pressure | 96 → 0 | 0 → 7559.0550899505615 → 0 |
| Pitch bend | 12,288 → 8,192 | 5000 → 7500.457763671875 → 5000.30517578125 |

Each message was verified by 13 matching active and three exact
neutral named DSP samples. The two other mapped controls stayed exact
in each window. After the original synth destination and disabled
monitor were restored, a full comparison found 2,623 matching
instrument DSP rows and 2,622 matching FX rows. The sole FX difference
was the MIDI bend center at `vstp516000001`. Both active and neutral
readings matched the documented test's Float32 normalizations scaled
to the host's raw range. No direct parameter setter was sent.

The earlier R462 attempt stopped after 27 requests and 24.975 seconds
before any routing edit or MIDI. Its fixture compared two actual
single-item operation results as objects. R462B matched the actual
arrays and passed the same isolated rack and source guards.

R462B made 212 requests in 111.695 seconds. Studio Pro, the owner,
and the temporary virtual source exited normally. The scratch song
was discarded without saving. The signed scanner bundle was retired,
and the 18 managed files and 11 app profile files were restored and
verified. The test establishes live control delivery in the stock
route. Physical input and the audible modulation of a loaded FX patch
require separate verification.
