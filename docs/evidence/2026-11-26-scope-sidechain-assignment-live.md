# Live Scope sidechain assignment ownership

R572B ran a private read-only diagnostic from `99f99f37` on Studio
Pro 8.1.1 Build 113103 for macOS arm64. One MCP owner was bound to
the isolated scratch song. Playback and recording stayed stopped,
and Main was muted at zero.

| State | Native and independent observations |
| --- | --- |
| Before assignment | The native diagnostic found the exact Scope Device and no source Send on two calls. Both mixer Send lists were empty. |
| Assigned | A single stock menu selection created one Send from the exact scratch source. The visible source checkmark and mixer Send destination agreed. On two independent diagnostic calls the retained port's two owner fields matched the separately bound Scope Device. |
| Released | The same stock menu removed the Send and the Scope header was switched off. The native result returned to `source_absent` twice. Both mixer Send lists were empty, and all 31 original Scope DSP rows matched. |

A wrong Send slot, source channel, Scope UUID and document were
refused. The diagnostic also required the current explicit document.
The only Send changes were the stock creation and removal. The
diagnostic reads did not change the route.

R572B issued 55 requests in 58.661 seconds. Studio Pro ran for
91.642 seconds. Both the host and owner exited normally without a
Save. The original song bytes matched, all 18 managed install files
and 11 changed profile files were restored, and no new cache members
remained. One earlier run stopped at the exact scratch-title guard
before a Send change.

The test verified the authored Scope assignment and the absent
source state in a sole-Send fixture. A public MCP read, general
connect or disconnect, and reciprocal processor attachment remain
separate work.

[Static retained-port evidence](https://github.com/dev-universe/fender-studio-pro-mcp/pull/345)
