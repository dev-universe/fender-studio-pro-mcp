# Serum 2 and Serum 2 FX Macro 1–8 in Studio Pro

R478 used the Xfer-signed Serum 2 2.1.5 VST3, Studio Pro 8.1.1 build
113103, and macOS arm64. The signed SDK catalogue listed Macro 1–8
for both classes at IDs 7000000 through 7007000 in increments of 1000.
Studio Pro reported the corresponding `vstp` parameter names and native
tags one greater than their VST3 IDs.
[Ableton MCP Serum control study](https://github.com/dev-universe/ableton-mcp/blob/36a4e7a/docs/serum-control-reference.md)

The isolated song contained one instrument and one FX insert on
separate tracks. Each of the 16 macro tests changed one exact DSP row
from 0 to 25 percent with `plugin_parameter_set`, then restored the
original raw Float64 bits. All 32 writes had matched readback. An
independent complete snapshot confirmed only the expected row moved,
and the other instance's 2,623 rows matched during the change. After
each restore, both complete 2,623-row sets matched their baselines.

The trial made 1,917 requests in 206.946 seconds. Studio Pro ran for
235.027 seconds and exited normally. The transport stayed stopped
and Main was muted at zero. No Save, playback, or MIDI was dispatched.
The temporary signed VST3 bundle was retired and 18 managed files and
10 app profile files were restored and verified.

This qualifies direct scalar macro changes on the observed patch
state. Modulation assignments and the audible effect of macros on
other patches require separate tests.
