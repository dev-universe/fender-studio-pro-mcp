# Inspect the active Serum 2 FX MIDI destination

Studio Pro 8.1.1 build 113103 on macOS arm64 with the notarized Serum 2
2.1.5 VST3 resolved the active Music track output to the exact Serum 2
FX insert. The verified fixture also contained an independently loaded
copy of that same class on a different channel.

1. Obtain the current `document` lease from `host_status`.
2. For the Music track, call `plugin_inspect` with `kind: "instrument"`,
   the exact `track`, `source: "dsp"` and `expected_document`. For the
   currently selected Serum 2 FX destination, the response identified
   an `AudioEffect` and its `native.instance.object_uid`.
3. Call `plugin_inspect` on the candidate insert with `kind: "effect"`,
   exact `channel`, zero-based `slot`, `source: "dsp"` and the same
   `expected_document`. The owner UUID must match the Music track
   response. The comparison in this trial was between two separate
   observations in the same document. Repeat the track observation
   with `expected_instance` set to the matching insert's UUID.

R468 used a read-only copy of the already saved fixture in a fresh app
process and document. The selected insert passed the explicit instance
check. The peer insert passed its own slot check but its UUID on the
Music track request returned `host_rejected`. A deliberately stale
`expected_document` returned `document_changed`. Both refusals returned
no result. A fresh read of the Music track and both insert chains
matched the initial observations. The saved archive stayed byte-stable.

The owner made 33 requests in 22.191 seconds. It issued zero writes,
Save commands, or MIDI dispatches. Studio Pro and the owner exited
normally. The temporary signed scanner bundle was retired and 18
managed files and nine app profile files were restored. This path
identifies the active destination through existing MCP tools. To change
the route in this verified workflow, use Studio Pro's stock track Out
control and read a fresh binding afterwards.
