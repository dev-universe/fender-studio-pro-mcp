# Recorded Scope sidechain Send creation

Studio Pro 8.1.1 build 113103 on macOS arm64. The trials used one MCP owner,
one protected scratch, stopped transport, Main muted and no active inputs.
The stock Scope Sidechain header was enabled before creation.

The recorded creation uses the exact sender's empty Send rack and the
independently retained and uniquely owned Scope input. The stock descriptor
layout, input facade, rack factory and unity-level operation were checked
against the installed arm64 image. Craft Anything independently matched nine
selected byte ranges to that same image. The edit adds a new Send at 0 dB.
The surrounding device and source rack are retained and checked again.

R578 used the trial and rejected a wrong Scope instance, stale document and
duplicate add. The verified add created one Send to the exact Scope. A single
stock Undo restored the empty rack. The trial made 69 requests in 71.089
seconds and the host exited successfully after 134.211 seconds.

R579 used the public core `d93980afa77f` with native UUID
`4D6D445A-2D66-33BF-947F-57C384C4E0DC` and fresh channel names. It used the
public read, create and remove tools in one session. The wrong instance,
stale document and duplicate creation were refused. Independent reads
confirmed one 0 dB unmuted Send, the exact destination, the bound removal and
empty Sends on both channels. The stock Undo menu recorded the removal and
creation separately. Undoing the removal restored the same Send UUID, level,
mute and routing. Undoing the creation then restored the original empty rack.
All 31 Scope DSP rows returned to the baseline.

R579 completed 78 requests in 115.223 seconds and the host exited successfully
after 164.377 seconds. The scratch closed without saving and its digest
matched. All 18 managed files and 10 app-profile files matched after
restoration. This trial covers the authored assignment, exact slot edits and
Undo. Signal delivery and cold reopening are separate observations.
