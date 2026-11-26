# Serum 2 FX Macro 1 to Distortion output level

R485 and R486 used the Xfer-signed Serum 2 FX 2.1.5 VST3 on macOS arm64.
REA with Ghidra and Craft-Anything found the same four strings in the
verified arm64 slice.

| Field or type | Mach-O address |
| --- | --- |
| `destModuleTypeString` | `0xDD38ED` |
| `destModuleParamID` | `0xDD7952` |
| `FXDistortion` | `0xDDDC2D` |
| `kParamLevelOut` | `0xDE5FE4` |

The strings identify the serialized field names. A preset from the
official package used `FXDistortion` module 0 and parameter 8 for
`kParamLevelOut`. The synthetic fixture retained one Distortion on the
Main bus and used source `[25, 0]` with amount −60. The comparison
fixture contained the same module with the modulation slot empty.

The isolated VST3 SDK host sent a continuous 440 Hz tone at −24 dBFS and
four points for parameter 7000000 through `IParameterChanges`. Each
condition rendered 96,256 stereo frames at 48 kHz. RMS values use the
settled portion of each window. Both channels measured the same at
the reported precision.

| Fixture | 0% | 50% | 100% | Back at 0% |
| --- | ---: | ---: | ---: | ---: |
| Route absent | 0.03702 | 0.03702 | 0.03703 | 0.03704 |
| Macro 1 to Distortion level | 0.03702 | 0.00593 | 0.00000032 | 0.03703 |

R486 applied the processor-only carrier to an exact Serum 2 FX insert
with `serum2_preset_apply` in Studio Pro 8.1.1 build 113103. The editor
showed one Distortion and its Matrix displayed `Macro 1 → FX 1: Dist Level`.
Two separate Rumble Native inserts measured stereo peaks immediately
before and after the FX. The same 440 Hz, −24 dB Tone Generator cycle
ran at every Macro position. Both sides produced the same results
at the reported precision.

| Macro 1 | Input median | Output median | Output/input |
| --- | ---: | ---: | ---: |
| Initial 0% | 668.1817 | 575.7873 | 0.86172 |
| 50% | 667.8411 | 92.1531 | 0.13799 |
| Restored 0% | 667.8975 | 575.7692 | 0.86206 |

The six paired reads per phase were within the pairing bound. MCP set
`vstp7000000` with `normalized: 0.5` and restored the original raw
Float64 zero. Both writes had matched readback. Macro 1 was the sole
changed DSP scalar, and all 2,623 rows matched after restoration.
The Tone Generator independently returned to Off and both stereo taps
returned to quiet after each phase.

The owner made 689 requests in 125.184 seconds. Studio Pro ran for
136.653 seconds and exited normally. Main and the test track were
silent at their physical outputs, the transport stayed stopped, and
the scratch closed without saving. All 18 managed files and ten app
profile files were restored. The temporary signed bundle was removed
from the scan path. This covers one FX route and the internal amplitude
response. Editable preset persistence and other modulation targets
remain separate tests.
