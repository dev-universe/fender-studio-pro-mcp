# Multi Instrument Filter Env Save-only control

In Studio Pro 8.1.1 Build 113103 on macOS arm64, a fresh byte copy of the
one-instrument scratch song had one owned Mai Tai slot, three graph nodes,
and two directed successor edges. The parent Filter Env read
`9299.99828338623` raw. Its sole assigned target, the exact owned Mai Tai's
`modulationTable.cell1.amount`, independently read `8180.276155471802`.
The original file's Filter Env pilot was `0.929999828338623046875`, which
matches the parent's raw bits after multiplication by 10,000.

With transport stopped and Main muted at zero, the exact scratch song was
saved once without opening the Multi Instrument editor, adding a child,
recalling a preset, or writing a plugin parameter. The resulting archive
had a new digest. All 20 serialized macro pilots and their assignments,
including Filter Env, matched the original archive. Its native graph
projection was exact. Two independent before/after live reads also matched
the full 150 parent and 283 child records, graph attachments, and mixer.

In the separate real Add and Save trial, the saved Filter Env pilot instead
matched the assigned original child. This Save-only control narrows the
observation to the editor Add path and its resulting state for the
tested fixtures. It does not identify the callback or establish a rule
for every Multi Instrument graph edit.

The control took 75 MCP requests in 17.564 seconds, with the app open
70.113 seconds. The app and MCP owner exited normally. The original and
once-saved copy matched their expected digests. Eighteen managed
installation files and nine app profile files were restored and verified.
