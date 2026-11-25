# Scope Sidechain destination lookup boundaries

Two private stopped trials used Studio Pro 8.1.1 build 113103 on macOS
arm64. Each opened the editor for the exact native Scope instance,
activated only its stock Sidechain header, and observed empty public
Send lists. Neither trial created a Send, chose a destination, or saved
the scratch song. Invalid source, slot, instance and document requests
were refused before the source-absent read.

R463C stopped because the native source rack contained more than one
direct node while its public Sends list was empty. Counting all native
children cannot determine the number of actual Send slots. R463D bounded
and retained the complete child sequence and separately counted exact
`AudioSendSlot` objects. It passed that check and reached the exact
Scope destination selector. The fixed getter call reached the host but
its result did not satisfy the required shared-object Variant check.
The candidate returned `host_rejected` without a scripting dialog.

The focused mixed-rack regression covers no Send, one Send among other
children, two Sends, and an untyped child. The R463D diagnostic feature
suite passed 511 tests. The live trials made 40 requests each, taking
20.104 and 30.175 seconds respectively. Each owner and Studio Pro exited
normally. Each trial restored and verified 18 managed files and 10 app
profile files.

The diagnostic remains private. A supported retained Scope destination
and the corresponding exact native Send assignment still need live
verification.

R463E added an independent `IListParameter::isEmpty()` check. The
operator invoked the stock Sources control and left the route unchanged.
The bounded native list reported empty before the selected value was
read. This trial made 40 requests in 25.592 seconds, repeated the clean
shutdown, and restored 18 managed files and 10 app profile files.
