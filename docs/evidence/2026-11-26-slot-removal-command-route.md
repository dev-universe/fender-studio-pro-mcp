# Studio Pro routed effect removal command and receiver

R567 inspected the Studio Pro 8.1.1 build 113103 arm64 executable.
The pinned slice SHA-256 is
`c6b326d8dfac4d468f8730508abcf8687997c6cd96a977a560600fda785a2e6e`.
R566 independently exercised the stock menu and Undo inside an owned song.

| Route | Static observation |
| --- | --- |
| Routing menu | The slot-node menu method at `0x100da57e0` attaches the `Device` / `Remove` command with the handler at the slot's bound object `+0x90`. `0x100da6ac4` registers its localized title alongside the ProcessGraph menu titles. |
| Exact handler | The `AudioInsertSlot` factory and constructor give primary table `0x101b78948`. Its command interface uses table `0x101b78fe8`. Interpret-command slot `+0x20` reaches thunk `0x100bafc20`, which adjusts the receiver by `-0x90` and delegates ordinary Device commands to `0x100d79830`. |
| Selection-dependent action | The Device / Remove branch separates the check-only flag from dispatch. Its dispatch consults the desktop handler and can queue `removeSelected`. The corresponding observer branch gathers selected devices before adding them to its operation. An exact menu handler pointer alone therefore gives no proof that an invocation deletes only one slot. |
| Per-slot action | The verified `AudioInsertSlot` observer table `0x101b78dc0` uses thunk `0x100baf52c` and handler `0x100baf200`. The latter delegates `removeSelf` to the inherited handler, whose exact receiver goes through primary slot `+0x230` to `0x100d7e50c`. Its non-deferred path resolves the owning folder and passes the receiver itself to the folder's single-device action at `+0x238`. |

REA with Ghidra 12.1.4 supplied the function bodies. LLVM disassembly
and Mach-O fixups pinned the thunks and table entries. Craft Anything
0.6.1 supplied 13 exact 24-byte windows that matched the same file. Its
local function index did not resolve the interpreter entry. The Ghidra
and Craft sessions were closed. Benchmark work took about ten minutes.

A future MCP edit needs a stable surviving anchor, the exact removed
slot and device identity, the owning folder, and the exact reciprocal
predecessor and successor immediately before dispatch. It must confirm
the native acceptance predicate and ensure no current selection is used
as an implicit target. This static unit did not call either route in the
app. The single-slot MCP mutation and independent recovery remain open.

[R566 live evidence](https://github.com/dev-universe/fender-studio-pro-mcp/pull/340)
