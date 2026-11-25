# Multi Instrument controller after preset recall

Studio Pro 8.1.1 build 113103 on macOS arm64 was tested with a single owned scratch song and MCP session. Playback was stopped, Main was silent, and the instrument track was disarmed with monitoring off during the controller checks.

| Trial | Native source | Independent result |
| --- | --- | --- |
| R322 | `d743ca67` | Factory recall changed the reported object ID from `Multi Instrument` to `Temp - 1 Mai Tai - Controls`. The native class and instance ID stayed the same. Ordinary inspection returned all 150 root parameters and the editor opened. Inspection using the `plugin_controller` scope was refused. |
| R325 | `23b8069` | The same title change retained the instance ID. Two separate `plugin_controller` reads returned the bounded controller report at the renamed title. A different instance ID was refused. The separate root read returned all 150 parameters. |

The native admission check now uses the exact Multi Instrument class ID, `AudioSynth` and a valid nonzero instance ID. Existing alias, session and document binding still apply. The regression test reproduces the renamed object and checks the report remains read-only. The native macOS arm64 release suite passed 498 tests.

Later in R325, two independent reads and the editor showed the two Mai Tai slots from a second factory preset. The visual acknowledgment timed out before the inverse recall, so the trial establishes controller read availability and the observed slot count only. Per-child DSP state, native audio routing and full preset restoration remain open.

R325 closed normally after the save was declined. All 18 managed installation files and their permissions were restored and verified, and the ten saved application profile files were restored. The host ran for 432.284 seconds.
