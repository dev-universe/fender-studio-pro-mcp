# Scope capture memory and the component stream

R436 examined Scope in the installed Studio Pro 8.1.1 Build 113103 arm64 `studioeffects` image. The fresh slice matched R420 and R431 byte for byte. UUID `CC2A5FBD-96A4-3231-9D30-DB782E03BE20`, SHA-256 `14eb3a7bc2a37b7867fcf68fbf1acd6537f4a3bd3431ce383fe26317e7801dca`. The check combined the prior REA/Ghidra results, Mach-O fixups, LLVM instructions and 24 fresh Craft Anything procedures.

| Path | Observed behavior |
| --- | --- |
| DSP storage | Three planar signed Float32 arrays hold up to 80,000 samples each. Acquisition fills them from a separate input history ring sized at roughly two seconds at the configured engine format. The configured capture window can be smaller than the allocation. |
| Trigger and fill | The producer records a trigger latch, ring-relative basis and collected count. The basis wraps with the input ring. A new trigger can restart the filled prefix while the previous tail remains. The inspected path has no absolute capture time or generation tag. |
| Editor | ScopeView consumes four arrays of 800 display values. It displays the signed peak per bucket and a scaled difference as the fourth trace. The display path has no synchronized read of raw samples or the completion counter. |
| Component Save and Load | The exact Scope `IStorable` interface delegates to the host effect preset settings stream. The Scope DSP save and load hooks and the controller save hook return success without transferring capture state. The controller load path updates input choices. Parameter delivery can reset capture. |

The exact DSP query and six internal attachment methods did not expose a synchronized status or waveform reader. The RemotePages entries and the public parameter interface expose settings. MCP’s component ownership and stream lock cover the component call and its byte buffer. The Scope producer has a separate audio-thread lifetime.

In the prior R381 live trial, two visibly distinct held traces produced the same 762-byte settings capsule. Loading the capsule cleared the visible hold. R432 verified progressive fill and a stable visible hold at Hold 200%. Those trials provide UI and settings evidence. A direct timing or sample read would require a verified host-owned coherent capture observation and an actual engine clock before adding any public reader. Restoring capture memory requires its own contract.

The 8-minute-11-second static check verified two ABI UUIDs, 15 fixups, 62 instruction anchors, 23 remote entries, 24 Craft procedures, and the R381/R432 records. No new DAW or MCP operation was performed. The separately tested settings-coverage field is proposed in [PR #172](https://github.com/dev-universe/fender-studio-pro-mcp/pull/172).
