# Impact factory preset recovery

Studio Pro 8.1.1 on macOS arm64 can apply Impact's `Drum Extraction` preset and recover the original `default` state through MCP. The verified procedure uses the exact Impact instrument and the native add-on with secondary output control. [Impact output control](https://github.com/dev-universe/fender-studio-pro-mcp/pull/23)

## Apply and recover

1. Bind the open document and exact Impact instrument. Record `channels_list`, the output inventory from `plugin_inspect` with `node_scope` set to `impact_outputs`, and the original parameter values. Use `plugin_state` to capture the component state. Keep the active document and native instance identity for each subsequent call.
2. List the instrument's presets with `plugin_presets`. Apply the issued index for `Drum Extraction` with the catalog token. Read back the preset display name and the output and parameter changes independently.
3. Load the captured component with `plugin_state`. Read the parameter values and output inventory again. The component values returned to baseline in the test while the four additional mixer channels were still active.
4. Inspect the added mixer channels, including inserts and sends. For an output created by the preset that has never held user edits, use `impact_output_set` to disable its exact index and name. Verify the remaining output flags and mixer rows after each change. In the test the four channels had empty inserts and sends and disabled recording and monitoring.
5. When the original display name was `default`, use the issued `default` preset if needed to restore the name. Compare the final mixer, parameters and a fresh component-state capture to the original. Release the state and catalog tokens.

## Studio Pro 8.1.1 verification

| Check | Observed result |
| --- | --- |
| Issued factory presets | `default` and `Drum Extraction` |
| R144 recall | `Drum Extraction` displayed and one writable value changed |
| Mixer recovery after component restore | 6, 5, 4, 3, then 2 channels using four MCP output changes |
| Final state | 4,262 writable values, the 35,153-byte component state's SHA-256, original mixer and `default` name matched |
| Test duration | 347 requests, 40.489 seconds of RPC work and 100.1 seconds including the app cycle |
| Preservation | Scratch song closed without saving. Studio Pro and the MCP owner exited normally. All 151 protected and 18 managed files matched. |

R116 separately applied the issued `default` preset after changing the pitch bend range. Two complete reads verified all 4,262 original writable values after recall and after restore.
