# Chorder input and note-off in Studio Pro 8.1.1

Two live Core MIDI inputs reached one Chorder Note FX on macOS arm64. MIDI 60 activated its five assigned output flags while held. The adjacent unassigned input 61 activated only its live input flag. Both returned to zero on note-off.

## Fixture and verification

- Studio Pro 8.1.1 Build 113103, diagnostic source `ea6390a318d7dd6df3627d7b7bb21a17a41148e6`, class `D2D9B002-12FC-4F1D-AFDA-70336D1DC108`. The named song had one Music track with Chorder and Mojito, no events or record arming, stopped transport, and Main at zero.
- The track was set to All Inputs. Studio Pro's external-device view showed one registered instrument MIDI input, bound to the virtual Core MIDI test source with no send port. A separate loopback verified the four generated note-on and note-off packets.
- With Learn mode off, a direct read verified input 60 mapped to outputs 60, 63, 67, 70, and 74. The stored model page reported no outputs for input 61. The MCP enabled track monitoring before the notes and returned it to off afterward.
- Input 60 at velocity 42 produced 17 reads of the active input flag and all five assigned `outputPlayingLive` flags. Input 61 at velocity 42 produced 16 reads of only its input flag. Both note-off messages were followed by two reads of all 26 watched signals at zero. The watched scheduled and GUI input signals stayed zero during the held notes.
- Afterward, two complete 908-row reads matched the baseline. The stored model and its opaque component capture matched their original hashes. The exact track, mixer, timeline and scratch archive were preserved.

The final client completed 129 requests in 19.827 seconds, and the app exited successfully after 55.599 seconds. All 18 managed installation files and 10 settings files, including the original input profile, were restored. The trial measures live input and Note FX output flags. Its scope includes neither physical audio nor edits to the learned chord.
