# Presence activation callbacks for all 16 script macros

Studio Pro 8.1.1 build 113103, macOS arm64. R348 loaded an owned SoundFont
in a scratch song, opened Presence's own Script Editor, and applied a
test script through the GUI. The script and sound were bound to the exact
Presence AudioSynth instance. The transport stayed stopped, Main silent,
and the instrument track unarmed and unmonitored.

The MCP set the eight Boolean macros to raw 1 and the eight continuous
macros to raw 1250 through 10000 in increments of 1250. After each
setter, independent control reads matched. Two complete reads matched
each other and preserved the other 225 rows. The script log separately
showed its aggregate activation marker, which required matching index,
name, and normalized value for all 16 callbacks.

A key tapped in Presence's built-in keyboard did not produce the script's
note marker in the observed view. The trial stopped before release or a
same-instance restore. Note behavior needs a routed input with an
independent witness.

REA with Ghidra resolved the ARM64 script method dispatch entries to
`0xc2920` and `0xc56d0` and the callback lookup to `0xc0334`.
Craft Anything 0.6.1 Deep independently decompiled both dispatch entries
from the same binary slice (UUID `EFDFFE66-D1E7-35E6-A0E2-FDD9C92B2C7E`).
Those results and the built-in reference distinguish the engine's named
zone lookup from the documented script note-discard method.

The scratch-open attempt before a host MIDI dialog was dismissed made
two read-only requests. The editor inventory then used 56 requests in
100.568 seconds and the macro activation phase 111 in 78.114 seconds.
The host exited normally after 250.617 seconds. No song was saved.
The original scratch bytes, 18 managed files and 10 changed profile files
were independently checked after cleanup.
