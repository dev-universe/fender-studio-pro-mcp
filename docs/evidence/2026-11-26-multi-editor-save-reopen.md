# Multi Instrument editor Add, Save, and reopen

On macOS arm64 with Studio Pro 8.1.1 Build 113103, an independently owned copy
of a one-child scratch song opened with one Mai Tai and a three-node native
graph. Transport was stopped and Main was muted at zero. The Multi Instrument
Routing editor's Add Instrument browser added one stock Mai Tai. The original
slot and live Device remained bound. Its 283 typed parameter records, the
parent's 150 records, and the existing mixer rows stayed exact. One mixer
channel routed to the Multi Instrument, and the newly attached child exposed
another 283 records.

The scratch copy was saved once from its own song window. Independent
inspection of the saved native component resolved seven unique nodes, six
directed successor edges, and two routes. The original slot UUID and its
ordered three-node route suffix were retained. A new shared splitter feeds
that original route and the new branch. Each route ends at its exact saved
slot. Two same-session reads after Save matched the graph, both child
bindings, and all 716 typed records captured before Save.

A new Studio Pro process and MCP session reopened the exact saved copy. The
editor showed the two parallel branches. Both live Device UUIDs were newly
bound. The saved logical node and slot UUIDs and their attachment
relationships matched two fresh readings. The stale document request was
refused. Both 283-record child providers and 149 of the 150 parent records
persisted exactly. The parent Filter Env changed from raw
`9299.99828338623` (display 86.0%) to `8180.276155471802` (display
63.6%). The first strict comparison stopped on this difference. A separate
MCP owner in the reopened app confirmed the single discrepancy, the visual
branches, and the old-document rejection without changing parameter values.

An archive comparison across all 20 macro definitions found one change.
The original Filter Env pilot was `0.929999828338623046875`. The pilot
written by Save was `0.81802761554718017578125`. The latter exactly
matches the normalized value of the one assigned `Inst01` parameter,
`modulationTable.cell1.amount`, and the reopened parent value. The new
`Inst02` was unassigned and had a different value for that parameter.
The discrepancy is already present in the saved archive. A Save-only
control on the one-instrument fixture remains to be run.

The Save witness took 134 MCP requests in 106.567 seconds. The strict reopen
and boundary confirmation took 49 and 94 requests in 4.500 and 15.271
seconds, respectively. Both DAW processes and all three owners exited
successfully. The reopened copy was closed without a second Save. The
original and once-saved copy matched their respective digests, and 18
managed installation files and 10 app profile files were restored and
verified.

The test qualifies the graph created by one editor Add and its persistence
in this fixture. Exact complete parent DSP persistence, a public MCP
instrument-add operation, and physical audio remain separate checks.
