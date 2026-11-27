# Chorder Copy, Paste and Clear All through MCP in Studio Pro 8.1.1

Three public `plugin_parameter_set` commands produced independently observed Chorder model effects on macOS arm64 Studio Pro 8.1.1 Build 113103. Each exact-instance DSP Toggle went from raw 0 to 1 and returned to 0.

| Parameter | Tag | Independently observed effect |
| --- | ---: | --- |
| `chorder.copy` | 11 | Copy in Learn Mode with MIDI 60 selected preserved the complete model. |
| `chorder.paste` | 12 | Paste into empty MIDI 61 added outputs 61, 64, 68, 71, and 75. All other inputs were unchanged. |
| `chorder.clearAll` | 10 | Clear All from that two-assignment model left all 128 inputs empty. |

- The actual host used the diagnostic build from `3c19e7a`, Chorder class `D2D9B002-12FC-4F1D-AFDA-70336D1DC108`, and one scratch song with stopped transport, no events or record arming, monitoring off and Main at zero.
- The MCP read the current learning key separately from the stored list. Two complete model reads followed both edges of each command. The lower edge preserved the upper edge's model result.
- Paste shifted the original MIDI 60 chord's five notes one semitone and increased the component from 581 to 787 bytes. Clear All reduced it to 360 bytes.
- One original component restore returned all 128 inputs and both hashes. The state token was released, the transient editor values returned, and two full reads matched all 908 original DSP rows.

The client completed 278 requests in 48.863 seconds. Studio Pro exited successfully after 74.533 seconds without saving the scratch. The exact song archive, 18 managed files and 9 modified app settings files matched after cleanup. These generic DSP writes were called within a bound test protocol. Their own receipts do not encode a selected-key or model revision guard.
