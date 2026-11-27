# Multi Instrument ownership across the bridge and EditTask

On macOS arm64 with Studio Pro 8.1.1 Build 113103, the native controller
report uniquely bound the selected Multi Instrument Device, its directly
owned Instruments folder and Graph objects, and the original Mai Tai
slot and Device. The live three-node graph and its slot attachment
matched the saved one-instrument scratch archive.

A read-only diagnostic queried four fixed paths rooted at the exposed
Device and Component scripting surfaces. All four folder observations
were absent. The Track EditTask could bind the enclosing Synths folder,
the instrument slot, and the exact outer Multi Device. The two fixed
lookups for its inner Instruments folder were absent. The document
factory returned a generic DeviceEditFunctions with an insertDevice
method. A request for the graph-specific edit functions returned no
object in that context.

The native controller still owned the same folder, graph and child
after those observations. Independent reads matched all 150 parent
and 283 child records, the mixer and the selected track. The observed
generic EditTask receiver provides no direct binding to the inner
Multi folder. A future native experiment can start from the exact
controller-owned node and must verify actual graph attachment and Undo
if it dispatches an insertion.

The diagnostic took 27 MCP requests in 3.678 seconds and the app ran for
50.628 seconds. The app and owner exited normally. The original and the
unsaved scratch copy matched their digests. Eighteen managed installation
files and nine app profile files were restored and verified.
