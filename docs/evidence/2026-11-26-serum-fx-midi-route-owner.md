# Serum 2 FX MIDI destination ownership in Studio Pro 8.1.1

R465 examined the exact arm64 Studio Pro 8.1.1 executable behind
[stock destination visibility](https://github.com/dev-universe/fender-studio-pro-mcp/pull/251)
and [virtual MIDI delivery](https://github.com/dev-universe/fender-studio-pro-mcp/pull/253).
REA with Ghidra covered 41 selected entry points and thunks. Seven
20-byte windows from Craft Anything independently matched the pinned
Mach-O. No DAW request, install, code patch or traffic was involved.

Studio Pro registers the AudioEffect's named AudioFXMusicUnit as a
distinct child. The effect constructs, class-checks and attaches it.
The receiver uses compiled host event-port routines. The Music
track's ordinary device selector and the connected instrument event
channel use separate host parameters. The named current-port getter
returns its current configuration as a retained object. This allows
the caller to hold the returned port during a bounded observation.

The inspected public port property branch returns the display label
and type. The host also has internal owner caches and class checks.
No supported public return from the inspected branch connects the
selected port's canonical identity to the exact AudioFXMusicUnit
attached to a particular Serum 2 FX insert. Equal product labels or a
verified FX insert elsewhere in the document cannot establish that
relationship. A practical qualifying fixture needs two independent
anchors, one from the selected Music destination and one from the
specific bound FX insert, with a same-model negative before publishing
an identity result.

The observed GUI selection and received values establish MIDI delivery
on the isolated route. This analysis leaves automated exact endpoint
binding and destination mutation open. No MCP operation was added.
