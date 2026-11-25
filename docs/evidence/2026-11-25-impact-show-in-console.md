# Impact primary output and Show in Console

R328 tested `Track / Show in Console` in Studio Pro 8.1.1 build 113103 on macOS arm64 with the `d743ca67` MCP build. It used a separate owned scratch, one owner, stopped transport, silent Main, and both Impact outputs disarmed with monitoring off.

The editor's dedicated output selector removed St 1 and the Edit menu recorded the corresponding output deactivation. One MCP Undo restored St 1 and its red color. The arrangement track remained selected and the detached Console selected St 2. Independent reads verified both active outputs, the 32 output rows and 4,262 typed DSP values.

The owner confirmed `Track / Show in Console` was available for the same document and executed it once. The target arrangement track remained selected, and the detached Console and two independent MCP reads still showed St 2 selected. Both output channels remained disarmed with monitoring off. A further full comparison retained all 32 output rows and 4,262 DSP values. The verified effect of this command did not include simultaneous restoration of the St 1 Console selection and the arrangement selection.

The trial used 252 MCP requests in 148.336 seconds. Studio Pro exited normally after 200.061 seconds. The scratch closed without saving. The original managed file hashes and modes, nine application profile files and the protected input files were verified after restoration.
