# Rumble Native factory recall and input/output restoration

Studio Pro 8.1.1 build 113103 on macOS arm64 recalled `028 KGB-800 CHORUS` at issued index 31 on one Rumble Native instance from the `001 BASIC BASSMAN CLN` baseline. The test used one owned scratch, with transport stopped and Main silent.

| Stage | Input | Output | Model and DSP observation |
| --- | --- | --- | --- |
| Baseline 001 | 0 dB | −11.00 dB | 39 typed DSP rows, 26 model slots and 22 numeric processor settings |
| Seeded input | −33.36 dB | −11.00 dB | Only the input parameter changed. The other 38 DSP rows and model and processor settings matched the baseline. |
| Recall 028 | 0 dB | −3.10 dB | The amp and `effect1` changed model, the input returned to the exact factory encoding, and 21 numeric processor settings were observed. |
| Recall 001 | 0 dB | −11.00 dB | All 39 DSP rows, 26 model slots and 22 numeric processor settings matched the original encodings and values. |

Each comparison used two independent reads. Both factory titles were visible in the editor. Guards refused a stale document and a different instance before dispatch. After release, the catalog and capsule tokens were refused and the baseline was unchanged. The only state recovery in this test was the factory recall.

This is the second exact factory-restoration case alongside `010 BASIC TV FRONT`. The [Rumble recall evidence](2026-11-25-rumble-factory-recall.md) records the separate 100-to-001 case with residual model identities in two empty slots and the prior float drift from generic capsule restoration. Audio, saved-state restart and complete coverage of the factory catalog remain unverified.

The MCP trial used 100 requests in 29.417 seconds. Studio Pro exited normally after 69.139 seconds. The scratch closed without saving. All 18 managed installation files and their permissions, ten application profile files and the original scratch bytes were verified after restoration.
