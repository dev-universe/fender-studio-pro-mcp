# Save and reopen after removing the third Splitter with two occupied lanes

Studio Pro 8.1.1 build 113103 on macOS arm64 opened an owned copy of
the verified three-Splitter song. Two exact MCP moves placed one
Mixtool in each lane of Splitter 3. The second move changed the two
numeric insert indexes while preserving both effect identities.
The stopped track and Main were muted throughout.

One guarded `insert_splitter_remove` produced the predicted 6-node,
7-link serial configuration. Both native effect and slot identities
and all 144 typed DSP rows matched the pre-removal providers. The
preparation and removal used 124 MCP requests in 78.001 seconds.

The operator issued one stock Save. The first comparison stopped
because its display-label rule assumed the original numeric insert
order. A separate owner compared the saved labels against the exact
native Device IDs, independently matched the saved graph via
`song-read`, and read the same warm DSP values. This recovery used
29 requests in 3.405 seconds and did not repeat any edit or Save.

A new process opened the owned saved file with writes disabled. The
ordered graph, both persistent native slot and effect identifiers,
and all 144 typed rows matched. The host issued new document,
folder and graph bindings. Two probes using the old document or
graph were refused before mutation. Fresh reads after both refusals
matched the reopened baseline. The cold owner used 49 requests in
4.956 seconds and issued no Save.

The two hosts ran 175.139 and 37.812 seconds and exited normally
with all owners. Sixteen captures were released. The original
fixture and the reopened saved file retained their verified
digests. The 18 managed installation files and nine profile files
were restored and checked.
