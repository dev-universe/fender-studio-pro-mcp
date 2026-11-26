# Ampire Black Panel Normal and Vibrato volume readback

Studio Pro 8.1.1 build 113103 on macOS arm64 loaded one stock Ampire
insert in an owned test song. The host was stopped, the stereo track
had no input, monitoring, sends or events, and the track and Main
were muted. The test bound the exact native AudioEffect and DSP
instance.

## Installed model evidence

REA used Ghidra on the exact installed arm64 slice. The Black Panel
model registers two separate volume parameters, `nvolume` and
`vvolume`. The stock Black Panel remote page resolves these as
`amp.nvolume` and `amp.vvolume`. The component definition declares
global `volume` and `pitch` separately. Craft-Anything independently
matched three 20-byte reads within the same slice to the original
file, including the two model registration sites. The analysis did
not assign a runtime target to either global control.

## Live verification

The model list and bound selection identified MCM 800 at index 1 and
Black Panel at index 4. Each local write was followed by two fresh DSP
snapshots. The paired local row and every other one of the 98 stable
rows were unchanged at the written step. The four provider meters and
the read-only tuner sensor were excluded from the stable row count.

| Control | Native tag | Display before, at change and after return |
| --- | --- | --- |
| Normal `amp.nvolume` | 40964 | 5.00, 10.00, 5.00 |
| Vibrato `amp.vvolume` | 40969 | 5.00, 10.00, 5.00 |

Both global controls retained raw 0 and empty display text in every
snapshot. Returning each local control restored all 98 rows and
the exact Black Panel component bytes. Returning to MCM 800 matched
its original 89 stable rows and component bytes by independent
capture. Five captures were released. No component restore command
was needed. Global alias attachment, signal behavior and save/reopen
are outside this trial. [Earlier stomp list observations](2026-11-25-ampire-volume-pitch.md)

A device-reconnect prompt initially prevented document binding. The
first owner ended without an edit after two requests in 9.041 seconds.
A fresh owner completed the trial in 99 requests and 9.576 seconds.
The owned host exited after 71.920 seconds with no Save, MIDI or
playback. The scratch archive remained byte-exact. The app and MCP
owner closed, and 18 managed files and 11 profile files were restored
and verified.
