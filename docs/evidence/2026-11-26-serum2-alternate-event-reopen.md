# Serum 2 and Serum 2 FX alternate event inputs after reopening

R516 opened four separate, Save-protected copies of Studio Pro songs
in a newly started Studio Pro 8.1.1 build 113103 process on macOS
arm64. The host loaded the Xfer-signed Serum 2 and Serum 2 FX 2.1.5
VST3s. Each document was closed before opening the next copy.

| Reopened copy | Stock Inspector destination | Independent native route |
| --- | --- | --- |
| R508 | Serum 2, Event Input 2 | Exact saved instrument UUID |
| R509 | Serum 2, Event Input 16 | Exact saved instrument UUID |
| R515 | Serum 2 FX, Event Input 2 | Exact first FX insert UUID |
| R514 | Serum 2 FX, Event Input 16 | Exact first FX insert UUID |

The independent read-only owner for each document compared the live
instrument or FX route to the saved Device UUID. The two FX documents
contained a second same-model insert. The owner read both insert
identities separately and verified that the Music route pointed to
the first, distinct receiver. CC 74 rows for channels 1, 2 and 16
all read zero on each restored receiver.

The stock track header showed arm and monitoring enabled on both
reopened FX Music tracks. This read-only trial left that UI state
as observed. Main was muted at zero throughout, and no MIDI
transmissions, playback, writes or saves were issued.

The four owners made 36 requests in 197.966 seconds. Studio Pro exited
after 217.978 seconds. Original archives and reopened copies remained
byte-identical. The temporary signed bundle was retired and 18 managed
files and nine app profile files were restored and verified.

[Serum 2 instrument archive evidence](https://github.com/dev-universe/fender-studio-pro-mcp/pull/287)
· [Serum 2 FX archive evidence](https://github.com/dev-universe/fender-studio-pro-mcp/pull/289)
