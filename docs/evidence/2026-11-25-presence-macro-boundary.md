# Presence script macro callback boundary

Studio Pro 8.1.1 build 113103, macOS arm64. R347 opened the Script editor
for a generated SoundFont on the exact Presence AudioSynth instance in a
disposable song. Transport stayed stopped, Main was silent, and monitor and
record arm were off.

## Editor and typed control inventory

- Two independent native reads returned the same 241 parameters. Eight
  `script.bool-1` through `script.bool-8` rows and eight `script.float-1`
  through `script.float-8` rows were writable and initially zero.
- After opening the Editor and Script panes, all rows, the whole serialized
  component fingerprint, the mixer and the timeline matched their prior
  values. Three state tokens were released.
- A second MCP owner bound to the same document, bridge session and exact
  native instance. Applying a small test script in the GUI changed the
  component fingerprint. The host script log displayed the initialization
  marker.

## Callback and zone readback

Separate controller reads after MCP writes returned `script.bool-1` raw
`1`, normalized `1`, and `script.float-1` raw `5000`, normalized
`0.5`. The Script panel displayed callback index `8`, name `float-1`
and value `0.5`. The visible log did not independently establish a Boolean
callback. The attempted `zone.tune` access returned `undefined` in the
log and the selected zone still displayed `0.00 st` in the Editor. The
trial ended at that verification gate before an in-session state restore.
A supported parameter setter and independent target readback remain open.

The editor and macro phases used 56 and 38 MCP requests and took 76.474
and 216.328 seconds, respectively. Both owners exited normally. The host
exited after 458.224 seconds. The scratch song was discarded without
saving and its original file was byte-identical. All 18 managed files and
10 touched application-profile files were restored.

[Presence Editor](https://fenderstudiopromanual.fender.com/en/Content/Built-In_Instruments_Topics/Presence_Editor.htm)
