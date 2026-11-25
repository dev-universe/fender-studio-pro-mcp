# Impact primary output and Console navigation

R324 used Studio Pro 8.1.1 build 113103 on macOS arm64 with source `d743ca67`, one owned scratch song and a single MCP owner. Main was muted at minimum gain, playback was stopped, and both output channels had record arm and monitoring off.

The Impact editor's output selector removed St 1 and left St 2 active. The Edit menu gained the specific undo entry for deactivating the Impact output. One bound MCP Undo restored St 1, its original red color and both active outputs. The arrangement track stayed selected, and the detached Console still showed St 2 selected. The independent read checked the 32 output rows and 4,262 DSP values after Undo.

The guard confirmed that `Console / Previous Channel` was available and dispatched it once. The detached Console and two separate MCP reads agreed that it selected St 1. The independent track reads also showed that the arrangement selection cleared. Both outputs stayed disarmed with monitoring off. Simultaneous selection restoration remains open, and this command is unsuitable as an automatic restoration step.

The trial used 207 MCP requests in 109.469 seconds. Studio Pro exited normally after 161.940 seconds. The scratch closed without saving. All 18 managed installation files and their permissions, nine application profile files and the pinned inputs were verified after restoration.
