# Pedalboard same-model readback after state reload

R340 used Studio Pro 8.1.1 build 113103 on macOS arm64 with the exact-build
diagnostic slot exchange. One Pedalboard held two Compressors. Slot 1 began
active with normalized Gain 0.324, and slot 2 began bypassed with Gain 0.683.
There was a Phase Meter after the rack. The transport was stopped and Main
was silent.

| Observation | Slot 1 Gain / active | Slot 2 Gain / active |
| --- | --- | --- |
| Independent DSP read before exchange | 0.324 / on | 0.683 / off |
| Independent DSP read after exchange | 0.324 / on | 0.683 / off |
| Independent DSP read after reloading the exchanged capsule | 0.683 / off | 0.324 / on |
| Independent DSP read after restoring the original capsule | 0.324 / on | 0.683 / off |

The rack editor showed the exchanged Gain and active indicators immediately.
Only the two `controls.active[stompN]` aliases changed in the first typed
read. The component fingerprint also changed. Reloading the captured
exchanged state once made the four slot-specific values agree with the
editor and preserved the exchanged fingerprint. Restoring the original
state once preserved its fingerprint and all 38 typed rows, and the editor
showed the original controls. The Phase Meter was independently checked
throughout. All five capsule tokens were released.

Static Ghidra and Craft-Anything analysis of the matching native slot
editor showed distinct paths for identical and different resolved model
classes. The identical-class path writes recorded attributes through the
slot-level setter. These observations establish a bounded same-session
recovery on an isolated rack. The reload is a native write. Undo behavior
and its interaction with live automation or playback were outside this
trial. Public slot-exchange admission remains limited to adjacent occupied
slots with different models.

The run used 96 requests over 29.138 seconds. The app ran for 185.020
seconds. The scratch song was closed without saving. Ten changed profile
files and all 18 normal managed files were restored.
