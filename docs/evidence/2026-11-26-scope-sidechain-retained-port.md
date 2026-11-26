# Scope's assigned sidechain input and its owning effect

R571 traced the Studio Pro 8.1.1 build 113103 arm64 image with
SHA-256 `c6b326d8dfac4d468f8730508abcf8687997c6cd96a977a560600fda785a2e6e`.
REA/Ghidra resolved 15 selected bodies. Twelve direct Craft Anything
0.6.1 byte reads matched the same image, including three fixup sites.
Both analysis targets were closed. This trial issued no host command.

| Binding | Static observation |
| --- | --- |
| Source Send assignment | The exact `PortParam::getCurrentPort` implementation at `0x100f0ec8c` reads its saved port and returns that object's `+0x30` interface. The scripting wrapper at `0x100f10830` stores it in an Object Variant and shares the result. |
| Sidechain table | The SideChainInput constructor in `0x100ba69e8` writes primary table `0x101b77470` and interface table `0x101b77730` at `+0x30`. Its canonical IUnknown is `+0x18`, which follows the CCL query path through `0x10159e5a0`. A match across those retained views bounds the primary receiver. |
| Independent destination effect | The routed AudioInsertSlot's native `+0x200` getter points at `0x100d75a0c`. It borrows the Device at slot `+0x1a0`. The stock editor's sidechain resolver uses the same getter. The exact Device can be retained and compared to the current MCP target before inspecting an assigned port. |
| Port's owner | The SideChainInput construction stores the exact effect in the base port at `+0x60` and in its notification owner at `+0x448`. The stock connection callback also reads the latter field to notify that effect. A comparison with a separately bound live Device can establish the destination without using the editor selector list. |

The narrow live test is a read of the source's sole retained Send and
the exact Scope Device before and after one stock sidechain assignment.
It needs a fresh document and device check, a stable shared port
reference, exact owner equality, and independent Send surface readback.
An empty source rack provides no destination Port observation. These
findings concern the authored assignment. A live reciprocal DSP
connection, an independent unassigned-input inventory, and a public
edit transaction require separate evidence.

[Earlier selector and retained assignment work](https://github.com/dev-universe/fender-studio-pro-mcp/pull/260)
