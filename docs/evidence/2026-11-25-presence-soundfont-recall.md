# Presence SoundFont program recall in Studio Pro 8.1.1

R345 used build 113103 on macOS arm64 in an owned scratch with one
Presence AudioSynth, no MIDI events and silent Main. Two SoundFont
2.01 files each contained a single newly generated sample. The Presence
editor's Load Preset File command loaded both files on the same instance.

| Stage | Displayed program | Displayed sample memory | DSP rows |
| --- | --- | ---: | ---: |
| Initial | No Program | 0 bytes | 241 |
| First import | MCP Sine 441 | 51.59 KB | 241 |
| Second import | MCP Triangle 220 | 103.16 KB | 241 |
| First file imported again | MCP Sine 441 | 51.59 KB | 241 |

The first import changed 39 DSP values from the empty program. The
second import changed the program and component fingerprint while
all 241 typed rows stayed equal. Returning through the first file
restored the UI program and DSP rows. The new serialized fingerprint
differed from the earliest capture of the same program.

Subsequent reads from a fresh MCP owner verified the same document,
service and native instance. One load of the original first-program
capsule returned its exact component fingerprint on two independent
captures, and the 241 typed rows and program display agreed. The plug-in
header showed its dirty marker. The mixer and timeline remained stable.
The owner released all seven tokens. The trial did not trigger MIDI or
measure audio.

The successful app run lasted 286.653 seconds and used 98 requests
across two serialized owners. A preliminary run stopped before import
at a test harness catalog bound of 128 rows; the independent reader
paged the full 241 rows in the successful run. Both scratch documents
were closed without saving and retained their original bytes. The ten
changed profile files and 18 managed MCP files were restored.

[Fender Studio Pro Presence manual](https://fenderstudiopromanual.fender.com/en/Content/Built-In_Instruments_Topics/Presence.htm)
