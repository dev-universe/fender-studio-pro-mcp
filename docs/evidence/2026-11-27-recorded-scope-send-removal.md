# Recorded Scope sidechain Send removal

Studio Pro 8.1.1 build 113103, macOS arm64. The host UUID was
`6B547378-B3BB-322A-87C9-901493FD2BED`. The trials used a single MCP owner,
stopped transport, no live input and a muted Main. Each trial used a protected
scratch song.

R574 traced the stock Send action with REA and Ghidra. The deletion branch
uses the same recorded per-slot operation as the stock insert slot editor,
with the exact AudioSendSlot in its own AudioSendRack. New Send creation follows
a distinct action. Craft Anything independently matched 14 byte ranges. R575
retained the Scope input before a Send existed, matched it to the assigned
port after one stock addition, then read it again after the Send was gone.
That trial made 55 requests in 49.270 seconds.

R576 confirmed that a wrong Send node never reached the native removal. The
initial core receipt treated the refusal as uncertain. R576B in a fresh
scratch reported the same rejection as `not_applied` and independently
confirmed unchanged Sends. An exact call deleted only the bound Send.
The stock Edit menu displayed a Send Slot Removal entry for that destination.
One Undo recovered the same Send UUID and mixer routing. The fixture then
restored the empty Send rack, Scope header and all 31 DSP rows.
R576B made 67 requests in 77.510 seconds and the host exited successfully.

The public operation requires the current document, two distinct exact audio
channels, the stock Scope instance at insert slot zero, stopped transport,
and the current Send UUID from the read operation. The source rack must
contain exactly one Send and its current destination must match the
independently owned Scope input. The result reflects host recording and the
bounded postcondition. Verify the assignment and mixer Sends independently.

## Public build verification

R577 used core `2ca7a102c61e` and native UUID
`C3EA656D-1C6D-3D11-ADA4-403C88B6CDED` with a fresh song and different
source and receiver names. Through the named public tools, a wrong Send UUID,
wrong Scope UUID, stale document and replay after removal all returned
`not_applied`. The successful call removed the single bound Send. Independent
reads showed empty racks, then the same Send UUID, level, mute and destination
after one stock Undo. All 31 Scope DSP rows matched the baseline after the
final stock cleanup.

R577 completed 74 requests in 72.539 seconds. The host exited successfully
after 109.552 seconds. The scratch closed without Save and its digest matched.
All 18 managed files and 10 profile files were restored or reverified.
The trial exercised one authored Send and the stock Undo journal.
