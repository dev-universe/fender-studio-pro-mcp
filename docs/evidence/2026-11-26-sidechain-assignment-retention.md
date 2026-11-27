# Retained Sidechain Send assignment candidate in Studio Pro 8.1.1

R460 rechecked the arm64 Studio Pro 8.1.1 build 113103 and the checked-in
CCL headers and adapter. It also compared the existing
[R419 live fixture](https://github.com/dev-universe/fender-studio-pro-mcp/pull/207)
and [R458 stock Send analysis](https://github.com/dev-universe/fender-studio-pro-mcp/pull/248).
No DAW requests were made.

The current channel owns a typed Send rack. An attached Send slot has a
named controller parameter whose fixed `getCurrentPort` message returns
the configured destination as a shared, retained object variant. The
ordinary Send label read does not return this object.

The already realized Scope editor Sidechain selector has a separate
list of inputs from its bound effect. Its selected item can carry a
borrowed interface. A compile-only CCL test validated the public
selected-value ABI and canonical object query before releasing that
variant. A candidate reader must hold the actual editor, effect,
controller and list through the read. Requesting the selector through
its ordinary lazy path can create a component. Only an already
attached selector qualifies for the read-only experiment.

The returned source and receiver objects use different interface
views. The comparison must query and retain canonical object identity
from both. The existing source route, new Send creation path and
corresponding reciprocal append were traced through the concrete
Send rack and slot. A matched identity would establish the configured
assignment. Reciprocal active membership and port index require
their own evidence.

Actual qualification remains open. It needs an independently resolved
receiver before the Send exists, one stock assignment, wrong-owner and
stale-document refusal, and an absent result after stock removal in
the same isolated song.
