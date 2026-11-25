# Rumble Native built-in browser and residual slot identities

R327 tested Studio Pro 8.1.1 build 113103 on macOS arm64 with the `d743ca67` MCP build. It used one owned scratch song, one owner, a single Rumble Native insert, stopped transport and silent Main. The issued MCP recall selected `100 A REMARK JACO MADE` on the bound instance, and two independent reads confirmed the changed model and processor state.

In the Rumble editor, `List View` opened the plug-in's own `MY PRESETS` list. Searching for `BASIC BASSMAN CLN` returned the single numbered `01` entry. Selecting it once updated the editor and the independent host catalog observation to `001 BASIC BASSMAN CLN`. No MCP preset apply was issued for this return to the baseline.

The 39 typed DSP rows and 22 numerical processor settings matched the baseline. Two independent reads and a follow-up after token release still reported different model IDs in `effect2` and `effect3`. These were the only mismatched model slots of 26. The baseline factory file declares both empty. This repeats the residual serialized state from the [MCP factory recall trial](2026-11-25-rumble-factory-recall.md) through the plug-in's own list. Whether the serialized IDs represent active processing remains unverified.

The trial used 88 MCP requests in 66.442 seconds. Studio Pro exited normally after 106.431 seconds. The scratch closed without saving. The saved scratch bytes, all 18 managed installation files and their permissions and ten profile files were verified after restoration.
