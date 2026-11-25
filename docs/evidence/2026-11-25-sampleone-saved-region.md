# SampleOne region after saving and reopening in Studio Pro 8.1.1

A song saved with one SampleOne waveform at Start 256 and End 12000 reopened with the same waveform and region in a new Studio Pro process on macOS arm64. The plug-in's 235 typed DSP rows and the SHA-256 of the opaque component matched the pre-restart captures.

## Fixture and procedure

- The diagnostic build came from `ea6390a318d7dd6df3627d7b7bb21a17a41148e6`. The exact instrument class was `C37BC9D1-6BD1-46A7-A60C-B13438666448`. One named scratch song, one client at a time, stopped transport, no timeline events, monitoring off and Main at zero.
- Loaded a generated mono PCM16 48 kHz WAV with 12,000 frames through SampleOne's sample browser. The visible start changed from 0 to 256 through the native editor. The visible end stayed at 12000. The component increased from 5,611 to 5,613 bytes, with a new SHA-256. The DSP row comparison returned zero differences.
- Saved the named scratch once and confirmed that the resulting song archive had a new SHA-256. Independent captures on both sides of the save matched the edited component and all typed rows.
- Closed the song and exited Studio Pro cleanly. A different process reopened the exact same saved archive. It reported a new bridge session and document, rejected a request containing the old document identity, and reported the same stored plug-in object UID.
- The editor showed the same waveform and Start 256 / End 12000. Two independent reads agreed on the 5,613-byte component and its SHA-256. All 235 typed DSP rows matched the saved-state reads.

The first client sent 68 requests in 189.891 seconds. The reopen client sent 30 requests in 16.189 seconds. Both host lifetimes exited successfully (211.011 and 49.663 seconds). The 18 managed installation files and 10 app settings files matched their original hashes after cleanup. The generated WAV was available at its original location throughout the trial. File relocation, missing-media handling and audio output were outside the trial scope.
