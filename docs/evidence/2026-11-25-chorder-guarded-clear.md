# Guarded Clear on a selected Chorder input

The new `plugin_action` variant `clear_chorder_selected` cleared one selected Chorder input in Studio Pro 8.1.1 Build 113103 on macOS arm64. It refused mismatched input pitch, model digest, object instance and document before the action's upper edge.

Read `chorder_chords_get` for the exact Chorder Note FX instance. In Learn Mode, pass the selected MIDI input and that read's lowercase model SHA-256 as `chorder_guard.expected_input_pitch` and `chorder_guard.expected_model_sha256`. Supply the current document, exact track, slot and instance as usual. The pre-dispatch checks are sequential on the host and the pulse describes its own upper and reset edges. Independently read the stored model afterward. The receipt is `applied_unverified`.

## Actual host evidence

- The candidate was built from `3c19e7a` with native arm64 UUID `F30796BD-3254-3B2C-9F5B-A20DED170DD4`. The test used a named scratch with stopped transport, no events, record arming and monitoring off, and Main at zero.
- The model contained the original chord at MIDI 60 and one generated assignment at MIDI 61. The exact selector reported Integer kind with a Float64 raw value of 62, which is MIDI 61 in this DSP representation.
- The four refused requests each returned `not_applied`. Two independent complete model reads confirmed that neither assignment and neither stored hash changed.
- Matching pitch, model, instance and document dispatched one upper and one reset edge. The receipt confirmed its reset, and independent reads found only MIDI 61 empty. The other 127 inputs and the original 581-byte component fingerprint matched.
- The original 908 typed parameters, exact mixer and timeline were restored. The app exited successfully after 73.204 seconds, with 169 requests taking 32.257 seconds in the single client. The scratch archive and all 18 managed files and 9 modified app settings files matched their originals after cleanup.

This trial covers selected-input Clear with the exact observed host and states. The scope includes neither Clear All nor audio output.
