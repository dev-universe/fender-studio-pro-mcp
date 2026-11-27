# Insert routing and Scope sidechain

These recipes use the [matching native add-on](INSTALLATION.md#optional-native-add-on)
on the verified Studio Pro 8.1.1 build. Bind the exact graph and current
document before editing. See the [workflow matrix](BUILTIN_PLUGINS.md#current-workflow-verification)
for the tested routing shapes.

## Inspect saved and live routing

For saved pre-fader insert racks with supported Splitter routing, `song_read` and `song-read` include `mixer[].insert_routing.nodes`. The ordered `successors` show where each node sends audio. Slot nodes include their saved rack index, slot ID and device name. The file reader reflects the last save. A warning identifies a rack whose links or slots could not be verified. [Studio Pro 8.1.1 verification](evidence/2026-11-20-saved-insert-routing.json)

For the current insert rack, use `insert_chain_state` with `action: "capture"` and `include_routing: true`. Supply the mixer channel and current `expected_document`. Use `expected_folder` to pin the exact folder from `plugin_inspect`. `routing.status` is `observed` when the host capture includes supported directed links, `absent` for a capture without graph metadata, or `unavailable` when the metadata cannot be verified. Slot nodes show the captured plugin class name and slot ID. Release the capture token when finished. [Live 8.1.1 verification](evidence/2026-11-20-live-insert-routing.json)

## Assign a Scope sidechain Send

Use `scope_sidechain_assignment_get` with two uniquely named audio channels and a stock
Scope in the receiver's insert slot `0`. Stop transport, read `host_status` for
`expected_document`, and read `plugin_inspect` for the Scope instance UUID. The
tool observes whether the source's sole Send is absent, unassigned, assigned to this
Scope, or assigned elsewhere. The verified native route supports up to 16 mixer-list
entries. The result describes the authored assignment only.

Use `scope_sidechain_assignment_create` to connect an empty source Sends rack
to that Scope at 0 dB. Enable the Scope Sidechain header and stop transport first.
Supply the exact channels, Scope instance and current document. Verify the Send
and assignment independently. Studio Pro records a Send Slot Addition in the
Undo history. A preceding mixer edit can finish recording after the Send, so
check the active Undo entry before using it.

Use `scope_sidechain_assignment_remove` to remove the sole Send assigned to that
Scope. Pass the same document, channels and Scope instance plus the read tool's
`send_node` as `expected_send`. Keep transport stopped. After the result, read the
assignment and mixer Sends again. Studio Pro records one Undo for the removal.

## Move, insert or remove a node

`insert_slot_move` moves an effect immediately after another slot in the same supported Inserts graph. Open that rack's Routing tab. Use `plugin_inspect` with `node_scope: "graph"` to get the graph, slot-node and effect UUIDs. Use a fresh capture with `include_routing: true` to identify the destination slot's single current successor. Supply the source channel, rack slot, plugin and instance UUID, plus the graph UUID, source node, destination node, and destination's current successor UUID. Send canonical uppercase UUIDs and the current `expected_document`. The move is available on the tested Apple Silicon build of Studio Pro 8.1.1 with the matching native extension. After `applied_unverified`, read the graph and plugin values independently. One host Undo restored the two parallel branches, all graph and effect IDs, and all 144 reported values in the nested Splitter trial. Studio Pro kept the moved effect selected after Undo. [Nested slot move and Undo verification](evidence/2026-11-20-insert-slot-move.json)

A separate save and restart trial kept both Mixtools in series in the inner Splitter branch. The reopened Routing screen, seven directed links and 144 reported values matched the saved song. [Saved nested slot move verification](evidence/2026-11-24-insert-slot-move-persistence.json)

To move an existing effect into an empty branch of a supported two-way Splitter, use the same tool with the Splitter UUID in `after_node` and its merger UUID in `following_node`. Set `branch` to `0` for the first branch or `1` for the second. In a fresh routing capture, the selected branch must lead directly to that merger. In the actual 8.1.1 trial, the second branch gained the effect, and one host Undo restored the original location, all graph and effect IDs, and all 144 reported values. Studio Pro kept the moved effect selected. [Empty branch move and Undo verification](evidence/2026-11-20-insert-slot-empty-branch.json)

The first-branch move was also verified through one Save and a fresh Studio Pro process. All eight nodes, ten ordered links and 144 Mixtool values matched, and the previous document binding was refused. [First-branch persistence](evidence/2026-11-26-splitter-first-lane-persistence.md)

After reopening a song, obtain a fresh document binding and read the current insert folder and graph UUIDs before editing. In the Studio Pro 8.1.1 restart trial, the saved links, slot and effect IDs, and all 144 values survived. The live folder and graph UUIDs changed, and the MCP rejected requests with the prior document or prior graph UUID before applying a change. [Save, restart and stale request verification](evidence/2026-11-20-insert-routing-reopen.json)

`insert_splitter_add` inserts a default two-branch Splitter and its merger on the existing link from a supported effect slot to its immediate merger. Both new branches start empty and rejoin before reaching the original merger. Open the exact rack's Routing tab. Use `plugin_inspect` with `node_scope: "graph"` and a fresh routing capture to get the effect and graph UUIDs. Supply `channel`, `slot`, `plugin`, `expected_instance`, `expected_graph`, the effect slot-node UUID in `after_node`, the immediate merger UUID in `following_node`, and the current `expected_document`. Use canonical uppercase UUIDs. The matching native extension checks that a recorded Undo is available on the tested Apple Silicon build of Studio Pro 8.1.1. After `applied_unverified`, inspect the new links and plugin values. One host Undo restored the original graph and all 144 Mixtool values in the nested trial. [Splitter insertion and Undo verification](evidence/2026-11-20-insert-splitter-add.json)

In Studio Pro 8.1.1, an added Splitter and both branches survived saving and restarting Studio Pro. The reopened rack matched all eight nodes, ten directed links, and 144 Mixtool values. Studio Pro issued new live document, insert folder, and graph bindings. Requests with the old document or graph were refused. [Splitter insertion persistence](evidence/2026-11-24-insert-splitter-add-persistence.json)

`insert_slot_remove` removes one effect between two adjacent effect slots in the same supported Inserts graph. Stop playback and recording, then read the graph and owner UUIDs with `plugin_inspect` using `include_nodes: true` and `node_scope: "graph"`, and take a fresh `insert_chain_state` capture with routing. Bind the surviving predecessor with its `channel`, `slot`, `plugin` and `expected_instance`. Supply the current `expected_document`, `expected_graph`, `preceding_slot_node`, `removed_slot_node`, `following_slot_node` and `expected_removed_instance`. Use canonical uppercase UUIDs. After dispatch, independently read the graph and effect values before issuing another edit. On the verified Apple Silicon build of Studio Pro 8.1.1, the operation removed the middle Mixtool in one branch. One native Undo restored its graph, effect identities and all 288 DSP values. [Exact slot removal and Undo verification](evidence/2026-11-26-routed-slot-removal-public.md)

`insert_splitter_remove` removes a supported inner Splitter and its merger. The other outer branch must lead directly to the outer merger. Supply both branch head slot-node UUIDs. By default each branch contains one effect. For a branch with exactly two effects, supply its second slot-node UUID as `first_branch_tail_node` or `second_branch_tail_node` according to the branch. Supply both tail UUIDs when both branches have exactly two effects. When one branch contains three effects and the other contains one, use `branch_continuations`. This is a pair of lists containing node UUIDs after each branch head in routing order. Use an empty list for the one-effect branch, such as `[[], ["SECOND_BRANCH_MIDDLE_UUID", "SECOND_BRANCH_END_UUID"]]`. The continuation form uses neither individual tail field. The surviving effects connect in series in first-branch order followed by second-branch order. Use `plugin_inspect` with `node_scope: "graph"` and a fresh `insert_chain_state` capture with routing to bind the graph and ordered branch nodes. Choose either branch effect as the anchor and supply its `channel`, `slot`, `plugin` and `expected_instance`. Also supply `expected_graph`, `splitter_node`, `merger_node`, `first_slot_node`, `second_slot_node` and the current `expected_document`. Use canonical uppercase UUIDs. The tested Apple Silicon build of Studio Pro 8.1.1 and matching native extension support the operation with the Routing editor closed. Check links and effect values independently after `applied_unverified`. One host Undo restored the original parallel branches, IDs and all 144 reported values in the Mixtool trial. Studio Pro selected the second effect after Undo. [Nested Splitter removal and Undo verification](evidence/2026-11-20-insert-splitter-remove.json) [Three-effect removal verification](evidence/2026-11-26-splitter-three-effect-mcp-remove.md) · [Second branch verification](evidence/2026-11-26-splitter-second-branch-mcp-remove.md) · [Both branches verification](evidence/2026-11-26-splitter-four-effects-mcp-remove.md) · [Three-effect branch verification](evidence/2026-11-26-splitter-ordered-continuations-mcp-remove.md)

The saved removal trial reopened the same song in a fresh Studio Pro session. The Routing editor showed the two Mixtools connected in series, and all 144 reported values matched the saved graph. Requests using the previous document or graph binding were refused before mutation. [Saved nested Splitter removal verification](evidence/2026-11-24-insert-splitter-remove-persistence.json)

`insert_splitter_remove_empty_nested` removes the third Splitter and its paired merger in the supported nested Mixtool graph. Both lanes of the third Splitter must lead directly to its merger. Stop playback and recording, open the rack’s Routing tab, and bind slot 0 Mixtool with `plugin_inspect` and a fresh routing capture. Supply `channel`, `slot`, `plugin`, `expected_instance`, `expected_graph`, `splitter_node`, `merger_node`, `predecessor_slot_node`, `successor_merger_node`, and the current `expected_document`. Use canonical uppercase UUIDs. Inspect the links and effect values independently after `applied_unverified` or an unknown outcome before attempting any further edit. In Studio Pro 8.1.1 for Apple Silicon, the two Mixtools stayed in parallel and all 144 reported values survived one Save and a fresh process. [Empty nested Splitter removal verification](evidence/2026-11-26-empty-nested-splitter-public.md)

Use `insert_splitter_remove_one_lane_nested` with the same slot 0
Mixtool and graph identifiers when one lane of the third Splitter is
empty and the other contains exactly one Mixtool. Add that
effect's slot-node UUID as `occupied_slot_node`. Stop transport and
inspect the graph independently after the request. In the verified
shape, removal kept both effects connected in series. One host Undo
restored the original topology and all 144 reported DSP values.
[First- and second-lane MCP verification](evidence/2026-11-26-nested-splitter-mcp-either-lane.md)

After `insert_chain_state` restores a captured chain, read the current routing and bind the current slot and plugin instance IDs again. Studio Pro may replace both IDs. The `applied_unverified` receipt confirms the host accepted the load. Compare the current connections and plugin values to verify the result. [Nested restoration verification](evidence/2026-11-20-insert-chain-restore.json)

In a separate Studio Pro 8.1.1 trial, one Save and a fresh process preserved the restored six-node parallel graph, both newly bound slot and effect IDs, and all 144 reported Mixtool values. An old-document restore request was refused before dispatch. [Restored chain persistence](evidence/2026-11-26-restored-insert-chain-persistence.md)
