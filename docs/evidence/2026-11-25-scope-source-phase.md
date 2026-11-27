# Scope channel C follows its DSP source in Studio Pro 8.1.1

R379B used Studio Pro 8.1.1 build 113103 on macOS arm64. An owned stereo
track contained Tone Generator, Mixtool and Scope in that order, with a
unique native binding for each insert. The transport was stopped, the
track had no input or events, and the track and Main faders were zero.
Scope's sidechain was disabled.

Tone Generator delivered a 440 Hz sine at −24 dB. Mixtool's independently
verified `phaseflipright` setting inverted the right channel. The Scope
editor showed opposite phases for the green Main Left and blue Main Right
reference traces. Separate vertical offsets kept both references and the
orange C trace visible together on an unchanged time scale.

| Bound `thirdSrc` readback | C selector caption | Orange C trace |
| --- | --- | --- |
| 2, Side Left | Main Right | Aligned with blue Main Right |
| 0, Main Left | Main Right | Aligned with green Main Left |
| 1, Main Right | Main Right | Aligned with blue Main Right |

The phase change establishes that the DSP setter switched the signal shown
by C even while its editor caption stayed fixed. Under this specific
sidechain-disabled configuration, raw 2 behaved like Main Right. The test
did not examine what raw 2 receives with a connected and enabled sidechain,
or settle every possible Scope source choice.

After Tone Generator was off, the original flat canvas and all 113 DSP
rows across three inserts were restored. Two final complete reads matched
the starting provider states, and the mixer and track projection matched.
There were 104 MCP requests in 60.859 seconds. The app closed without saving
after 88.468 seconds. The scratch bytes, 18 managed installation files and
10 app settings files were verified against their originals.
