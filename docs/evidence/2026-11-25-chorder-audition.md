# Chorder audition hold and release in Studio Pro 8.1.1

The lower C3 key in the actual Chorder editor activated the five mapped live output flags while held. The separately tested `chorder.playLearnChord` DSP toggle reported its own state while the watched output flags stayed zero.

- Tested macOS arm64 Studio Pro 8.1.1 Build 113103, Chorder class `D2D9B002-12FC-4F1D-AFDA-70336D1DC108`, and diagnostic source `ea6390a318d7dd6df3627d7b7bb21a17a41148e6`. The test track had no events, arming, or monitoring. Transport was stopped and Main was zero.
- With Learn Mode on and MIDI 60 selected, MCP set the exact DSP tag 8 to 1. Four independently bound reads found the flag at 1 and all five watched live and scheduled output flags at zero. The flag returned to 0. The original GUI mode, model and all 908 DSP rows were restored.
- With Learn Mode off, the editor's lower C3 key was held for three seconds. Fourteen independently bound reads found the GUI input velocity at 8000.00011920929, the live input flag at 1, and output-live flags at 1 for MIDI 60, 63, 67, 70, and 74. The standalone preview and scheduled output flags stayed at zero in that episode.
- The audition's release returned all watched signals to zero. Two full reads matched the original 908 rows, all 128 stored mappings and their model and component hashes. The mixer and timeline were the same.

The DSP and final editor clients completed 127 and 148 requests in 39.839 and 19.747 seconds, respectively. A preliminary editor read helped establish that the GUI input carries velocity. Studio Pro exited cleanly after 180.897 seconds without saving the scratch. The scratch archive, 18 managed installation files and 9 changed settings files matched their originals afterward. This evidence covers flags and state, with audio kept silent by the test setup.

Fender describes audition from the lower keyboard in its [Note FX manual](https://fenderstudiopromanual.fender.com/en/Content/Built-In_Instruments_Topics/Note_FX.htm).
