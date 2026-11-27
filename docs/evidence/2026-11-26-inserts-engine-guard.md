# Insert graph publication and read lifetime

R448 inspected the exact macOS arm64 Studio Pro 8.1.1 build 113103 executable
and the installed CCL system library. It checked 21 Ghidra functions, 14
instruction anchors, nine rebases, eight independent Craft Anything byte
ranges, and two CCL interface or class UUIDs. All results in this report are
from static inspection.

The engine uses one CCL exclusive lock to publish its current compiled graph.
The audio callback attempts the same lock before reading the published graph
and processing the block. A failed attempt can cause that callback to skip
graph execution. A replacement graph is prepared and then swapped under the
engine lock. The old graph can be detached and released during a replacement.

The insert graph's own rebuild obtains a lock through the attached
`AudioTaskProcessor` and its `ILockProvider` interface. The interface can
return a borrowed explicit lock or inherit its lock from the attached parent.
An unattached task can return no lock. The compiler can clear and release
each adjacency allocation and rewrite both arrays in place on the same
processor object. A retained object protects its identity. The array lifetime
requires the correct synchronization domain as well.

A caller would have to prove that the current Routing target resolves to the
active published engine owner and its lock across the relevant writers. The
examined processor getters returned individual pointers or counts. The named
graph snapshot action is an edit and Undo operation. An immutable two-sided
processor view and a safe read entry point remain unverified.

R443C independently observed eight authored nodes, ten ordered occurrences
and both incoming occurrences at one Merger in the actual host. That result
supports an authored topology description of source UID, successor ordinal
and destination UID. The two repeated peers remain distinct occurrences.
Processor timing, physical ports and generalized edits require separate
evidence.
