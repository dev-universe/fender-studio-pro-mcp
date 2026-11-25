# Pedalboard same-model payload across save and restart

R339 used Studio Pro 8.1.1 build 113103 on macOS arm64 and the same diagnostic
admission as the [two-Compressor exchange](2026-11-25-pedalboard-same-model.md).
Both slots held Compressor. Slot 1 began active with normalized Gain 0.324.
Slot 2 began bypassed with normalized Gain 0.683. The two slots had the same
model class and distinguishable settings.

| Stage | Slot 1 Gain / active | Slot 2 Gain / active |
| --- | --- | --- |
| Original independent DSP read | 0.324 / on | 0.683 / off |
| Same-session DSP read after the exchange and after Save | 0.324 / on | 0.683 / off |
| Saved native rack | 0.683 / off | 0.324 / on |
| Fresh app and independent DSP read | 0.683 / off | 0.324 / on |

The editor's Gain knobs and bypass indicators switched immediately. During
the original session only `controls.active[stomp1]` and
`controls.active[stomp2]` changed in the 38-row DSP catalog. The component
fingerprint also changed. The saved `.dsppreset` independently placed both
Gain and active payloads in the exchanged positions with the same Compressor
class ID. After restart, both slot-specific DSP rows and the active aliases
agreed with that exact saved payload. The other 32 rows and the Phase Meter
peer were preserved. The new session refused the old document reference.

This confirms same-model exchange and persistence through restart in the
diagnostic scope. Immediate slot-specific DSP readback remains stale in the
original session. The public adjacent-swap admission still requires different
models.

The two app runs totaled 111.656 seconds with 82 and 17 MCP requests. Only
the owned scratch was saved once. Its saved bytes were preserved and the 18
managed files and ten changed profile files were restored.
