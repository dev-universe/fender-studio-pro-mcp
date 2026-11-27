# Rumble Native factory topology and the Add Block catalog

The Rumble definition bundled with Studio Pro 8.1.1 build 113103 on macOS
arm64 declares an input stage, tuner, 25 effect slots, amp, and output.
Its 26 processing orders put the first `n` effect slots before the amp
at `global.amplocation = n`. The rest follow the amp. The tuner and
amp have separate bypass roles. The amp selector targets the amp
simulation class group and the 25 effect selectors target the stomp
class group. These are declared roles in the installed resource.

The 111 bundled factory presets collectively reference 17 distinct
nonempty amp model identifiers and 38 distinct effect model identifiers.
Their saved amp positions cover 0 through 4. These counts describe the
factory files. The selectable runtime domain may differ.

R389 opened Add Block in a fresh scratch song on that build. The editor
advanced to Select Node To Add and then Add, where no model rows or
category labels rendered. The trial canceled the picker with no model
confirmation and no direct DSP write. This repeated the earlier empty
picker in R135. The file-based census and the live factory recall
evidence identify models, but individual Add Block selection remains
unverified on this installation.

The 25 MCP requests took 37.676 seconds and the host ran for 76.642
seconds. The scratch closed without saving and remained byte-identical.
Both processes exited normally, and all 18 managed files and ten profile
files were restored. The underlying vendor resource and preset files
remain local.
