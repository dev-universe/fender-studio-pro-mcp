# Pedalboard and Ampire pre/post placement

R320 used Studio Pro 8.1.1 build 113103 on macOS arm64. A scratch audio
track held the exact standalone Pedalboard and Ampire effect classes. Both
editors showed Compressor, Reverb, and Big Fuzz in their first three slots.
The standalone Pedalboard editor showed a continuous rack. Its 40 non-meter
settings contained no `opt.stomporder` row. Its installed DSP definition
ordered the eight stomp slots between input and output.

Ampire showed the blue pre/post divider between slots 4 and 5. Its installed
DSP definition listed nine amplifier positions. The live `opt.stomporder`
row had integer values from 0 to 8. One exact-instance parameter request
changed the raw value from 4 to 1. A separate DSP read and the editor both
placed the divider between slots 1 and 2. A second request restored raw 4
and the original divider position. All 102 non-meter Ampire settings and the
original component fingerprint matched on restoration. The standalone
Pedalboard's 40 settings and component fingerprint matched before and after.
Four read-only meter rows per plug-in were excluded from setting comparisons.
[Fender Studio Pro Ampire manual](https://fenderstudiopromanual.fender.com/en/Content/Built-In_Effects_Topics/Distortion.htm)

The divider affects Ampire's amp placement. The trial exercised its position
via `plugin_parameter_set` and observed the standalone rack. The public
Pedalboard slot exchange targets the standalone effect class. The test
observed editor and processor state with Main silent.

The trial used 93 MCP requests in 99.943 seconds including visual checks.
The host exited normally after 141.111 seconds. The scratch song was closed
without saving. Its original bytes, 18 managed MCP files, and ten restored
Studio Pro settings and cache files were checked.
