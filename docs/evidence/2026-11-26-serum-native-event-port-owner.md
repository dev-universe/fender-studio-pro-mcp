# Native Event Input ownership in a mixed Studio Pro song

R535 traced the installed Studio Pro 8.1.1 build 113103 arm64 process
image with REA/Ghidra, the Mach-O rebase records, and Craft-Anything.
The executable UUID was `6B547378-B3BB-322A-87C9-901493FD2BED`, and
11 Craft windows matched the original extracted bytes across the
initial and corrected analyses.

The input-port object exposes `label` and `type` to scripts. Its native
interface selected by IID `7D3D7A65-028B-4E65-BF43-B57FC16955C9`
has a distinct owner accessor. On the qualified runtime interface
table at `0x101bc2a70`, slot three rebases at `0x101bc2a88` to
`0x100d9b4ec`. The function obtains the stored owner and returns its
interface. The diagnostic retained both ports, verified the exact host,
interface table, and function, then compared the owner's identity
within the same native call. The receipt contained a Boolean.

The first two trials refused every comparison because the prototype
initially pinned the base-class table. R535B isolated the runtime table
from the failure, and the separate Mach-O rebase check confirmed the
same owner accessor. R535C repeated the test with the corrected pin in
a fresh stopped scratch song. Two Xfer-signed Serum 2 VST3 2.1.5
instruments and one stock Mai Tai produced 33 distinct ports.

| Port group | Same owner as Serum A | Same owner as Serum B | Same owner as Mai Tai |
| --- | --- | --- | --- |
| 16 numbered inputs on Serum A | Yes | No | No |
| 16 numbered inputs on Serum B | No | Yes | No |
| Synth Input on Mai Tai | No | No | Yes |

Both independent scans agreed, for 198 comparisons. A track-channel
argument refused the port interface query. The two Serum instance
identities and all 5,246 DSP rows matched after the read. R535C made
158 requests in 39.103 seconds. Each trial closed without Save, MIDI,
playback, or a route change. The three scratch copies, all 18 managed
files and 10 affected profile files per trial were verified after
restoration. The tested native helper was private. A public route
setter and the FX owner path need their own live tests.
