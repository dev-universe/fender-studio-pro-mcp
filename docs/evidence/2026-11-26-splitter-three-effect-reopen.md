# Saving and reopening a three-effect Splitter removal

R556 exercised the extension in [PR #329](https://github.com/dev-universe/fender-studio-pro-mcp/pull/329)
in Studio Pro 8.1.1 build 113103 on macOS arm64. Three individually
bound Mixtool effects started in two occupied branches of Splitter 3.
The first branch had two effects and the second branch had one.

One `insert_splitter_remove` request supplied the exact first branch
tail and returned `applied_unverified`. Independent live reads showed
seven nodes, eight ordered links, and serial effect ordering. All
216 typed DSP rows and the three effect identities matched the
pre-removal values. Studio Pro saved the owned project once.
Two more live reads matched the saved graph. An offline `song_read`
matched the archived node IDs, ordered links, slot IDs and instance
labels to the warm readback.

A separate Studio Pro process reopened the saved file under a
verified write guard. Its new native session, document, graph and
folder IDs bound the same saved effect IDs and 216 DSP rows.
The previous document and previous graph were rejected independently
before dispatch. Two fresh reads after the refusals again matched the
saved graph and DSP values. The archived file hash stayed the same.

The warm owner issued 210 requests in 95.073 seconds and released
16 captures. The cold owner issued 55 in 5.497 seconds and released
four captures. The processes exited normally after 124.185 and
33.134 seconds. All 18 managed installation files and ten changed
profile files were restored, and no capture files remained.
