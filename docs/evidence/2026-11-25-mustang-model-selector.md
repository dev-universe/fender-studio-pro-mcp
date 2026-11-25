# Mustang Native model selector readiness

R344 used Studio Pro 8.1.1 build 113103 on macOS arm64. The exact
Mustang Native AudioEffect was attached to an owned empty scratch
session. Main was silent and transport was stopped.

The normal DSP inspector reported 39 parameters. The bounded
controller observation returned partial metadata for the exact
component. Both current-selector catalogs reported zero rows with the
editor closed and on two independent reads with the Add selector open.
The editor showed the stock preset title and an empty chain. After
opening Add Block and choosing an insertion position, the Add panel
contained no model rows. No Confirm or model write was issued.

During the same app launch the native loader reported a missing code
signature for the installed arm64 Mustang model module. A separate
strict code-signature check of that file also found it unsigned. These
observations establish the current installation's selector readiness
and loader failure. The test did not change any vendor files and
cannot establish the selector behavior on an installation where the
module loads successfully. The current Fender manual describes the
normal model and block-selection UI.
[Fender Studio Pro manual](https://fenderstudiopromanual.fender.com/en/Content/Built-In_Effects_Topics/Distortion.htm)

The app ran for 60.853 seconds. The MCP owner used 31 requests over
29.493 seconds. The scratch file remained byte-identical after closing
without a save. Ten changed profile files and 18 managed MCP files
were restored.
