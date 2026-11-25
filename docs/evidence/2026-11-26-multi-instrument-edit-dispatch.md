# Multi Instrument editing dispatch in Studio Pro 8.1.1

The installed macOS 8.1.1 Build 113103 executable's arm64 slice matched the
SHA-256 of the slice inspected with REA and Ghidra. The native metadata and
call sites identify three separate parts of an instrument insertion.

- The generic DeviceFolder descriptor declares `insertDeviceClass`, with
  a class UID, an optional position, and a DeviceSlot result. The script
  wrapper and the generic DeviceEditFunctions insertion enter the same
  folder-dispatched native operation.
- The exact CombinatorSynthFolder class has its own dispatch entry for that
  operation. It enters the base insertion and conditionally calls a second
  method on the model bound to the returned slot. The live Multi controller
  previously exposed that folder as its own `Instruments` child alongside
  a separate `Graph` object.
- The build registers SynthCombinatorGraphEditFunctions as a specialization
  of DeviceEditFunctions. It also registers separate graph edit actions and
  graph side effects. The existence of a DeviceSlot result alone does not
  establish event routing or an Undo transaction.

For a prospective MCP operation, bind the live Multi parent, an exact
direct synth folder and its existing slots before dispatch. Independently
read the new slot, its bound Device and its graph attachment after
dispatch. Keep saved successor edges, Undo and unaffected DSP values as
separate acceptance checks. The actual editor Add produced all of these
graph observations across the Add/Undo and Add/Save trials, including a
separate persisted parent Filter Env discrepancy. The callable bridge
receiver and the equivalence of a script call to the real editor operation
remain open. This inspection did not execute an insertion.
