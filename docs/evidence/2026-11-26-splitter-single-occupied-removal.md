# Removing a nested Splitter with one occupied lane

On macOS arm64 with Studio Pro 8.1.1 build 113103, R500 prepared a
scratch rack with Mixtool 2 on the second lane of Splitter 3 and the
first lane empty. A single document-, instance- and graph-bound
`insert_slot_move` call set up that topology. Routing and independent
native reads confirmed eight nodes, ten ordered links, both unchanged
Mixtool identities and all 144 DSP rows.

The stock Routing menu's **Remove** command was selected once for
Splitter 3. The editor showed Mixtool and Mixtool 2 in series where the
inner branch had been. Independent native reads confirmed six nodes
and seven links. Exactly the selected Splitter and its merger retired.
Both Mixtool devices stayed in the original folder, with the same
identities and DSP values.

The enabled **Undo Splitter Remove** item in Studio Pro's Edit menu
was selected once. The editor showed the original nested Splitter with
Mixtool 2 in the second lane. All eight nodes, ten ordered links,
native child enumeration order and 144 DSP rows matched their
pre-removal snapshots. Mixer, sends and the empty timeline stayed
unchanged. This is evidence for the Studio Pro menu action.

R500 made 173 requests in 141.526 seconds and released 14 temporary
capture tokens. Studio Pro ran for 179.961 seconds. The scratch closed
without Save, the app and MCP owner exited normally, and 18 managed
files and nine profile files were restored and verified.
