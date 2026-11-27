# Scope note-trigger input and capture read boundary

R553 traced the macOS arm64 `studioeffects` image from Studio Pro 8.1.1
build 113103. The slice SHA-256 was
`14eb3a7bc2a37b7867fcf68fbf1acd6537f4a3bd3431ce383fe26317e7801dca`.
REA with Ghidra resolved 35 selected functions. Ten 24-byte windows from
Craft Anything 0.6.1 Deep matched the original slice. LLVM disassembly
and Mach-O rebases established the dispatch and field offsets.

The Scope DSP constructor installs the class table at `0x5064d8`.
Its `+0x170` method resolves to `0x331ec`, which reads the first
event bus and asks each event for the observed `8Note` record type.
The handler checks for a zero type field and a nonzero floating value,
then tests the configured trigger gates before writing a ring-relative
offset at DSP `+0x19c0`. The next store writes a one-byte event flag
at DSP `+0x19c6`. The handler derives the offset from the event offset
and the selected ring cursor with one wrap adjustment.

The wrapper at `0x236c90` dispatches the Scope audio leaf at `0x33414`.
The leaf passes DSP `+0x1470` as the capture context to acquisition.
This makes the event flag and ring-relative offset visible to acquisition
as fields `+0x556` and `+0x550`. In the event-trigger branch the
producer checks the flag, tests the existing latch, copies the offset,
sets the latch, and resets the capture count before appending samples.
These stores are distinct from the later fill-count and hold checks.

The separately inspected notifications on this DSP table use the base
configuration fields at `+0x1451` and `+0x1454`. Their payload construction
does not use the Scope acquisition latch or capture count. The ScopeView
query at `0x471c0` returns retained view subobjects for its two explicit
interface IDs. Those replies contain no capture-generation record.
The note path therefore supplies a producer input, leaving coherent
status and waveform readback open. This static trial did not run the
DAW or prove that Studio Pro exposes Scope as a Music-track destination.
