# Scope channel C source display in Studio Pro 8.1.1

R377 used a new stereo track in an owned scratch song on macOS arm64,
Studio Pro 8.1.1 build 113103, and the exact Scope instance of class
`36322412-AD90-4EE0-B63C-37929566B081`. The track had no input or events.
Transport was stopped, arming and monitoring were off, the track and Main
faders were at zero, and the Scope editor showed sidechain disabled.

| Observation | DSP `showThird` | DSP `thirdSrc` | Visible C selector |
| --- | ---: | --- | --- |
| Initial | 0 | 2, Side Left | Main Right |
| C enabled | 1 | 2, Side Left | Main Right |
| Main Left set through DSP | 1 | 0, Main Left | Main Right |
| Restored | 0 | 2, Side Left | Main Right |

The `showThird` change turned on the orange C strip and its flat trace,
while the selector caption stayed fixed. The named MCP read with `source`
set to `auto` could read `thirdSrc` and `showThird`. It reported the skin
names `srcchan1`, `srcchan2` and `srcchan3` as missing in all four states.
The exact caption mismatch therefore persisted with C enabled and after
an accepted Main Left DSP selection in this editor session. This test did
not establish which source a non-flat C trace would actually represent.

Two final independent reads matched all 31 original DSP rows. The mixer
and test track were unchanged after the source transition. The owner
completed 42 requests in 51.335 seconds, and the host exited in 85.454
seconds. The scratch remained byte-identical after close without saving.
The 18 managed installation files and 10 application settings files were
restored. A separate editor-side selection and an actual signal routed to
the relevant channels are required to establish the mapping.
