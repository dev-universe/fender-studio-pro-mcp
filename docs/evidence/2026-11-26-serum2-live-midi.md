# Serum 2 instrument virtual MIDI in Studio Pro 8.1.1

R459 used the Apple-notarized Xfer Serum 2 2.1.5 instrument VST3 and
Studio Pro 8.1.1 build 113103 on macOS arm64. The test independently
matched the signed mapped plug-in, one current document and native
instrument, and one receive-only Core MIDI virtual input by its name
and ID. The stock track inspector showed that input and the Serum 2
destination. The owned track's monitor was on during the messages, Main
remained muted at zero, and the transport stayed stopped.

Each first-channel test sent one changed MIDI value and one neutral
value. These are independent named DSP reads of the already verified
[reference assignments](2026-11-26-serum2-midi-reference.md).

| Message | Input active → neutral | Raw DSP baseline → active → neutral | Active reads at observed value |
| --- | ---: | ---: | ---: |
| CC 1 | 96 → 0 | 0 → 7559.0550899505615 → 0 | 13 |
| Channel pressure | 96 → 0 | 0 → 7559.0550899505615 → 0 | 14 |
| Pitch bend | 12,288 → 8,192 | 5000 → 7500.457763671875 → 5000.30517578125 | 13 |

Both seven-bit reads match the Float32 normalization of 96/127 scaled
to the host's 0–10000 value. The bend values match the Float32
normalization of 12,288/16,383 and 8,192/16,383 at the same scale.
The two peer mapped controls stayed exact during each test. The original
full-restoration test stopped on the pitch-bend center difference after
28 neutral reads. The final monitor change read back independently.

A fresh read-only owner in the same host session and document after the
virtual source closed found just one changed row among all 2,623.
`vstp516000001` remained at the transmitted MIDI center. The instrument
can therefore retain a hardware-style nominal center that differs from
its initial host value. These results add actual MIDI delivery evidence
to the separate [direct scalar writes](2026-11-26-serum2-mapped-parameter-writes.md).
Serum 2 FX MIDI routing and physical input are outside this test.

The MIDI measurement made 160 MCP requests in 56.565 seconds. The
follow-up made 38 requests in 4.741 seconds and issued zero writes.
No note, transport command, direct parameter write, or song Save
was dispatched. Both owners, the virtual MIDI source, and Studio Pro
exited normally. The temporary signed scanner bundle was retired and
18 managed files and 11 app profile files were restored and verified.
