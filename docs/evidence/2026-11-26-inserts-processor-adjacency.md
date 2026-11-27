# Insert graph and compiled audio processors

R447 inspected Studio Pro 8.1.1 build 113103 for macOS arm64. Instruction
checks, loader rebase checks and independent byte samples matched the same
executable slice. This trial made zero DAW or MCP requests. R443C supplies
separate live evidence for the eight-node, ten-edge nested fixture, including
two outgoing occurrences from one Splitter to its paired Merger and two
reciprocal incoming occurrences.

## Splitter and Merger

The host compiler resolves each eligible graph node to its owning `AudioTask`
processor. It copies all outgoing occurrences into the source processor's
output list and all incoming occurrences into the destination processor's
input list. Each list is built in the order of its corresponding logical
list. Duplicate peers survive both copies. A successful compilation of the
R443C graph therefore predicts two references to the same Merger processor
in the Splitter's output list and two references to the same Splitter processor
in the Merger's input list. This prediction has no live processor-list read
yet. The fixed Splitter/Merger pairing is a single reciprocal association
regardless of the number of outgoing occurrences.

During audio processing the Splitter selects a buffer by its output ordinal
and delivers it to the corresponding destination processor. The Merger counts
deliveries within the block and emits the combined buffer once it reaches the
compiled input count. The source ordinal and the two processor identities
describe a semantic edge within one compiled topology. The counter resets for
each block. A persistent destination port ID remains unverified. A read of both
lists would need a host guard covering compilation and the attached processors'
lifetime.

## Sidechain port ownership

The stock Select Sidechain Source action resolves the selected source channel
and its Send context, then resolves the destination port independently from
the selected effect. The effect's Sidechain input has an owner-local index.
At the lower-level port interface, connect and disconnect check endpoint type
and exact membership and update each endpoint's own list. The specialized
input also publishes notifications. The full call chain from the Send property
to that lower-level update, atomic rollback, and live reads of both endpoint
indices remain unverified. R419 separately verified a typed Send to Scope,
its editor witness and restoration.

These findings support further read-only graph verification. Public support
remains scoped to the existing documented graph operations.
