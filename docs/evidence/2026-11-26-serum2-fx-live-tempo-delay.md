# Serum 2 FX delay follows Studio Pro tempo

R549 verified the Xfer-signed arm64 Serum 2 FX 2.1.5 VST3 in the
local Studio Pro 8.1.1 Build 113103. A diagnostic instrument
generated the same 10 ms stereo burst in every playing window.
The exact Serum 2 FX instance was the first insert. A diagnostic
FX in slot 1 recorded the first sample above 0.0001 absolute
amplitude and the peak at its audio input. Main was at zero.

The public `serum2_load` tool inserted the signed plug-in.
`serum2_preset_apply` accepted three processor-only states on the
same native instance while stopped. Independent inspection
confirmed the same native class and instance and 2,623 DSP rows.
The receipt bound the input hash, document and exact instance.
The downstream audio confirmed the audible behavior.

| State | Delay from dry at 90 BPM | Delay from dry at 147 BPM |
| --- | ---: | ---: |
| Dry | 0 samples | 0 samples |
| Tempo-synced eighth note | 15,998 samples | 9,795 samples |
| Fixed 250 ms | 11,950 samples | 11,951 samples |

All six windows used 48 kHz and 512 frames per block. The signal
peak ranged from 0.549096 to 0.549470. The observed delays were
within two samples of the signed processor's independent SDK
renders for the same states. [Independent comparisons](https://github.com/dev-universe/fender-studio-pro-mcp/pull/322)

The initial comparison required the preset receipt to repeat
the entire native inspection object. The receipt supplies the
instance and class UUIDs. A fresh owner confirmed those UUIDs,
the original SHA-256, and the same session and document. It
played the existing dry state without applying it again, then
applied the two remaining presets once each.

The two owners made 25 and 95 requests in 3.616 and 12.307
seconds. Six Play/Stop cycles completed. Studio Pro exited
normally after 111.063 seconds without Save. The temporary
bundles were moved out of the scan path after rehashing.
All 18 managed installation files, ten changed profile files
and the original scratch bytes were verified after restoration.
