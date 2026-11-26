# Direct MCP Clear on a selected Chorder input in Studio Pro 8.1.1

The public `plugin_parameter_set` operation on the exact Chorder DSP Clear parameter removed the selected chord. Its own flag and the independent stored-model observations both changed. Releasing the flag left the selected chord empty.

- macOS arm64, Studio Pro 8.1.1 Build 113103, diagnostic source `ea6390a318d7dd6df3627d7b7bb21a17a41148e6`, Chorder class `D2D9B002-12FC-4F1D-AFDA-70336D1DC108`. One named scratch had stopped transport, no events, monitoring and record arming off, and Main at zero.
- The exact open editor was in Learn Mode at MIDI input 61. The existing MCP learning tool assigned it output 65. Independent 128-input reads showed the original five-note chord on 60 and the test chord on 61.
- With the document, instance, class and DSP provider bound, two reads observed the `chorder.clear` Toggle at tag 9 and raw 0. A single raw 1 write produced two reads of 1. Two complete stored-model reads then showed MIDI 61 empty, MIDI 60 and the other inputs unchanged. The component returned from 659 bytes to the original 581 bytes and SHA-256.
- A scoped raw 0 write released the flag. The complete model still matched the cleared state. Original component and transient learning settings were verified after cleanup, with all 908 DSP rows equivalent to the original. The original state token was released.

The client completed 179 requests in 30.704 seconds. Studio Pro exited successfully after 73.576 seconds without saving. The scratch archive, all 18 managed installation files and 9 changed settings files matched their original hashes. This confirms the selected Clear effect through the existing MCP tool in this learning context. A generic flag write does not itself enforce the selected input pitch or stored-model revision.
