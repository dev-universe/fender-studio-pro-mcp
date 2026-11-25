# Chorder Copy and Paste transposition in Studio Pro 8.1.1

Copy preserved all 128 stored Chorder inputs. Paste shifted the five copied output notes by one semitone when the target moved from MIDI 60 to MIDI 61. Only the target input changed.

- Studio Pro 8.1.1 Build 113103 on macOS arm64, diagnostic source `ea6390a318d7dd6df3627d7b7bb21a17a41148e6`, Chorder class `D2D9B002-12FC-4F1D-AFDA-70336D1DC108`. The single test track was stopped and unarmed, with monitoring off, no events, and Main at zero.
- Learn Mode showed MIDI 60 (C3) and outputs 60, 63, 67, 70, and 74. Clicking the editor's Copy button once preserved the complete native model and original 581-byte component hash.
- Selecting the empty MIDI 61 (C♯3) also preserved the model. Clicking Paste once produced outputs 61, 64, 68, 71, and 75 on input 61. Two independent model reads confirmed the other 127 inputs were identical, including the source. The component was 787 bytes, with a new SHA-256.
- One exact-instance component restore returned the original 128 input mappings and both hashes. Learn Mode and the selected pitch returned to their original transient values. Two complete reads matched all 908 DSP rows, and the token was released.

The client completed 179 requests in 113.665 seconds. Studio Pro exited successfully after 147.959 seconds without saving the test song. The archive, 18 managed installation files and 9 modified settings files were verified after cleanup. The buttons were actuated through the native editor, and MCP verified and restored the result. Clipboard implementation and audio output were outside the runtime measurements.

The workflow is described in [Fender's Note FX manual](https://fenderstudiopromanual.fender.com/en/Content/Built-In_Instruments_Topics/Note_FX.htm).
