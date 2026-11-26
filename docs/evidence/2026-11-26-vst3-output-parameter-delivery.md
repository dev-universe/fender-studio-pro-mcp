# VST3 output parameter delivery in Studio Pro 8.1.1

R454 inspected the exact Studio Pro 8.1.1 build 113103 macOS arm64
executable and its installed VST service. The bundle's signature passed
strict verification and was ad-hoc. The extracted VST service slice matched
the previously captured R197, R319 and R446 slices byte for byte. REA with
Ghidra examined 34 functions. Craft Anything independently matched 18 byte
ranges, alongside ten loader rebases and four SDK interface identifiers.

The stock insert task provides a backing list for VST3 output parameter
changes. Its VST3 adapter translates the parameter ID to the host ID and
copies the Float64 value into an output point along with its sample offset.
The host drains accepted points into an owned queue after the processor
call. The consumer resolves the current hosted parameter, converts the
value to Float32 and uses the point's timing and flags for immediate or
scheduled delivery to the host parameter proxy. The inspected adapter and
queue path supplied no blanket normalized-value range test. Invalid or
unresolved targets can still be discarded.

The separate host edit-controller endpoint converts the host ID and calls
the installed controller's `setParamNormalized`. The final connection
from the output-driven host refresh to that endpoint and the controller's
actual response were outside the verified call chain. A stopped transport
and delayed readback do not identify which output points ran through it.

In [R304](2026-11-24-fabfilter-legacy-readback.json), seven FabFilter
instances returned the tested legacy control to zero. A prior isolated
processor observed zero output for six of the seven and a negative output
for Pro-G. Its test driver only delivered points in the normalized interval.
The actual Pro-G point, its host disposition and its controller response
remain unverified. This static observation leaves all seven positive
retention checks open. It made zero live DAW calls.
