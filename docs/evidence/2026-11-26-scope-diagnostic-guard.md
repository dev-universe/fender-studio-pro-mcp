# Scope diagnostic guard in Studio Pro 8.1.1

R463 and R463B used isolated scratch copies in Studio Pro 8.1.1 build
113103 on macOS arm64. A feature-gated diagnostic tried to compare one
fixed source Send and one exact Scope input. Its internal request
needed a source object. The shared JavaScript wrapper originally
rejected that request shape before reaching native instance validation.
The 29th request timed out while Studio Pro showed a scripting dialog.
Both earlier invalid source and slot requests returned `host_rejected`.

A test through BridgeCore, BridgeComponent, and NativePluginControls
reproduced the original error. The private correction admits only the
fixed request when its native capability is advertised. It verifies the
native Scope identity before accessing the selector or a Send. Six
focused test cases passed, including rejection of unsupported shapes
and stale state and preservation of the existing preset object rules.

With the rebuilt private diagnostic, the exact corresponding 29th
request returned `host_rejected` with an instance mismatch and no
scripting dialog. The slot and source still returned `host_rejected`,
and stale and absent document preconditions produced `document_changed`
and `missing_precondition` respectively. The owner reached the exact
Scope editor. The stock source menu could not be confirmed visually,
so the trial stopped before creating a Send. R463B recorded 33 requests
in 76.598 seconds. It issued no Save. Studio Pro exited normally and
all 18 managed files and 10 app profile files were restored.

Live verification of a successful Send assignment and removal remains
open. This record covers the diagnostic guard and its observed failure
path. The diagnostic and its private fixture were not added to MCP.
