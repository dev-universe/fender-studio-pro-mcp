# Studio Pro slot removal and its recorded action

R569 traced the stock `Device / Remove` command on the Studio Pro
8.1.1 build 113103 arm64 image. The pinned slice SHA-256 is
`c6b326d8dfac4d468f8730508abcf8687997c6cd96a977a560600fda785a2e6e`.
Eight REA/Ghidra bodies resolved, and 13 direct Craft Anything
0.6.1 byte reads agreed with the image. The investigation closed both
targets and ran no Studio Pro or MCP host command.

| Stock step | Image evidence |
| --- | --- |
| Bind the edit context | The inherited `removeSelected` observer at `0x100d7c9e8` constructs an in-place 80-byte edit helper at `0x100d6c290` using the receiving AudioInsertSlot. It retains the action context found through that receiver. |
| Start and finish the group | The observer starts the native group with an empty host string through `0x100d6c924` and finishes the edit helper through `0x100d6c580`. The destructor closes the active operation and clears the helper's scoped state at the outermost depth. |
| Target one slot | The observer gathers selected slots and calls `0x100d6d27c` once per selected primary slot pointer. The per-slot helper accepts an explicit slot pointer directly. |
| Select the recorded path | The helper checks the owning folder flags, the slot flag and the host edit predicate. The recorded branch constructs one action for the exact slot, formats the stock title and calls edit-helper virtual slot `+0xe8`. The other branch calls the slot's direct virtual removal, which R568E observed leaving the preceding move at the top of Undo. |
| Validate journal ownership | Edit-helper field `+0x40` holds the action context. Its `getActionJournal` call is used by the recorded submitter at `0x101316d9c`. If no journal is available, that submitter can execute the action directly. |

An MCP trial should verify the exact slot and both neighboring edges,
the owning folder, the inactive host scope, the retained action
context and its journal, and the branch conditions before one
per-slot call. It should then check independent graph and DSP reads
and one native Undo. The evidence establishes the stock calls and
their branching, with a live trial of the journaled exact-target
variant still open.

[R568E direct-removal result](https://github.com/dev-universe/fender-studio-pro-mcp/pull/342)
