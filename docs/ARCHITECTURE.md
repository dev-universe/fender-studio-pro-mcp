# One executor owns the DAW

The MCP client reads the active document, then sends typed edits with that
document token. The executor returns what it applied, what it verified, and
anything whose outcome remains unknown.

One native owner thread owns the CoreMIDI source, mailbox and the complete
write-and-readback sequence. A profile lock rejects a second controller process.
MCP handlers submit bounded requests and never write mailbox files directly.
Rust domain operations are finite. A raw arbitrary script endpoint is not part
of the API.

Bridge protocol 2 carries the request ID, expected bridge session and document
token. The DAW checks the context immediately before edits. A document token
tracks the active document's lifetime. Reopening a saved path does not revive
old references. A response must match the request and bridge session.

A timeout after dispatch can mean the host edited the session but the reply was
lost. The executor reports an unknown outcome and does not repeat non-idempotent
edits. Repeated MIDI wakeups for one request ID are deduplicated in the bridge.
This is not crash-safe exactly-once execution.

The initial implementation has one crate and one owner. A multi-client broker
can later own that same executor if needed. It is not a prerequisite for real
DAW verification.

Two independent same-model design candidates converged on this structure.
Candidate A was selected for its concrete finite operation boundary. Candidate
B contributed lifecycle-specific outcomes and plugin acceptance covering audio
effects, instruments, Note FX and non-parameter operations. A separate
same-model judge scored A 24/25 and B 23/25. These are design judgments, not
runtime measurements.

All-plugin support requires per-class evidence from the actual DAW. Local
factory maps provide parameter candidates, not proof of complete enumeration.
The runtime reports missing parameters and control limitations explicitly.
