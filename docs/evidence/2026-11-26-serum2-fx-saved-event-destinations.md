# Saved alternate event destinations for Serum 2 FX

R514 and R515 used the Xfer-signed Serum 2 FX VST3 2.1.5 in Studio
Pro 8.1.1 build 113103 on macOS arm64. Each scratch song contained a
Music track and two separate stereo Audio tracks, each with the
same signed plug-in in insert slot 0. The Music track targeted the
first FX instance. The second instance was loaded after the route was
selected. Both receivers and Main were muted. Transport, arm and
monitoring were off, and no MIDI was transmitted.

| Trial | Route at first native Save | Saved destination port |
| --- | --- | --- |
| R515 | Event Input 2 | `MusicInput01` |
| R514 | Event Input 16 | `MusicInput15` |

In both files, the Music track's saved destination owner UUID matched
the first Audio track's exact FX device UUID and the live native
instance. The other FX had a different device UUID. Studio Pro
displayed the second insert as `Serum 2 FX 2`. The independent
archive and live checks used exact identities for both. All 5,246
typed DSP rows, the track state and mixer projections matched
before and after the one normal Save.

Each trial made 143 requests. R514 took 52.680 seconds and R515 took
78.688 seconds. The apps and owners exited normally and the temporary
signed bundles were retired. All 18 managed files and 10 profile files
per trial were restored and verified. The initial R513 fixture stopped
before Save because it expected the same display name for both
instances. Its changes were discarded.

The saved reader matches these two ports for the exact Serum 2 FX
insert class. CLI and stdio checks compare six real saved archives.
The result identifies the correct Audio channel and slot in all three
FX archives and all input files remain byte-identical.

[Live alternate-port delivery](https://github.com/dev-universe/fender-studio-pro-mcp/pull/288)
