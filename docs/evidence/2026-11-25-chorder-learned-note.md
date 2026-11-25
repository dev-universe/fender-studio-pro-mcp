# Chorder learned-note round trip through MCP in Studio Pro 8.1.1

On macOS arm64 Studio Pro 8.1.1 Build 113103, the public `chorder_learning_set` tool added and removed one output note on an initially unassigned Chorder input. Two complete native model reads per checkpoint distinguished the GUI receipts from the actual stored chord changes.

- The named test song contained one Chorder on a Music track with stopped transport, no events or record arming, monitoring off, and Main at zero. The exact Chorder class was `D2D9B002-12FC-4F1D-AFDA-70336D1DC108` and the diagnostic source was `ea6390a318d7dd6df3627d7b7bb21a17a41148e6`.
- The MCP opened the exact Note FX editor. The initial GUI learning mode was off with sentinel input `-1`. The original 128-entry model contained only the five-note chord assigned to MIDI 60.
- The bound GUI operations enabled Learn Mode and selected MIDI 61. Two full stored-model reads confirmed that selection alone did not alter the chord model or component hash.
- The `chord_note` operation required the exact selected input 61 and enabled MIDI output 65. The receipt marked the GUI phase `closed_checked` with outcome `applied_unverified`. Independent native reads found exactly `61 → [65]`, a new model hash, and a component size change from 581 to 659 bytes. The other 127 inputs were identical.
- Disabling that one output note restored all 128 inputs and both original hashes. Turning Learn Mode off and clearing the selected pitch restored the original transient GUI state. Two complete typed reads matched all 908 original rows, and the mixer and timeline matched the baseline.

The client completed 143 requests in 25.730 seconds. Studio Pro exited cleanly after 59.470 seconds. The scratch archive was unchanged and all 18 managed installation files and 9 modified app settings files were restored. This trial covers an in-memory edit and reversal. No song save or audio output was measured.

The corresponding editor workflow is described in [Fender's Note FX manual](https://fenderstudiopromanual.fender.com/en/Content/Built-In_Instruments_Topics/Note_FX.htm).
