# SampleOne sample import and empty-state recovery

R363 used Studio Pro 8.1.1 build 113103 on macOS arm64. The exact
SampleOne AudioSynth instance was opened on an empty, unarmed,
unmonitored instrument track with the transport stopped and Main silent.

Two generated mono 16-bit 48 kHz WAV files were added through the
instrument's own file picker. The first waveform showed 12,000 frames
and 46.91 KB in the sampler. With both files loaded the list showed both
entries and 140.69 KB. Selecting the second entry showed its own
waveform and 24,000-frame end. Neither file was from an existing
user project.

| Stage | Component bytes | Stable typed rows |
| --- | ---: | ---: |
| Empty instrument | 5222 | 235 |
| One wave | 5601 | 235 |
| Two waves | 5942 | 235 |

Both imports changed the serialized component hash while the 235
typed DSP rows all remained the same. One native restore of the
original capsule cleared the sample list and waveform in the editor.
Two independent DSP reads and a new serialized capture matched the
complete initial state. Mixer and arrangement were preserved.
The title retained its dirty indicator after the restoration.

The owner made 70 calls in 107.089 seconds and released four tokens.
The app exited normally after 136.636 seconds. The song was discarded
without saving and remained byte-identical. All 18 managed files and
10 touched app-profile files were restored. This trial covered import
and same-session recovery. Region edits, save/reopen, MIDI input and
audio-output measurement are separate.
