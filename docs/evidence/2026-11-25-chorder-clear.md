# Chorder Clear and Clear All in Studio Pro 8.1.1

The two Chorder editor buttons had different effects on two assigned inputs. Clear removed the selected chord and preserved its neighbor. Clear All removed both. Independent MCP reads verified all 128 stored inputs at each checkpoint.

- Studio Pro 8.1.1 Build 113103 on macOS arm64. The exact Chorder class was `D2D9B002-12FC-4F1D-AFDA-70336D1DC108` and the diagnostic source was `ea6390a318d7dd6df3627d7b7bb21a17a41148e6`. The single test track had no events or record arming, monitoring off, stopped transport and Main at zero.
- The original model had the five-note chord on MIDI 60. The public learning tool added output 65 to MIDI 61. The editor showed MIDI 61 (C♯3) selected in Learn Mode, with its one output highlighted.
- Clicking Clear once removed only MIDI 61. The complete model and component hash then matched the original map, and MIDI 61 stayed selected.
- After reconstructing the same two assignments, clicking Clear All once removed both. All 128 inputs were explicitly empty and the component was 360 bytes, with a new SHA-256.
- A single exact-instance component restore brought back the original 581-byte model and its original hashes. After the token was released, a fresh client bound to the same document and instance independently rechecked the model, restored the transient GUI controls, and verified all 908 typed DSP rows against the baseline. Neither button was repeated during cleanup.

The two clients completed 153 and 64 requests in 51.473 and 6.298 seconds. Studio Pro exited cleanly after 133.650 seconds. The test song archive, all 18 managed installation files and 9 modified app settings files matched their original hashes after cleanup. The buttons were actuated through the native editor. MCP is the observer and restoration path in this trial.

The editor buttons are described in [Fender's Note FX manual](https://fenderstudiopromanual.fender.com/en/Content/Built-In_Instruments_Topics/Note_FX.htm).
