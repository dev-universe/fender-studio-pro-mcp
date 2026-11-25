# Scope capture data and host lifecycle

R451 inspected the exact macOS arm64 Studio Pro 8.1.1 build 113103 host
and stock Scope binary. The static observations matched 22 Ghidra functions,
39 instruction anchors, nine loader rebases, five strings and 17 independent
Craft Anything byte ranges. This inspection made zero live host requests.

The Scope processing callback forwards this block's size and input/output
lane descriptions. The capture routine receives its retained capture context.
The callback path supplied no absolute frame or device timestamp to that
routine. Acquisition updates its filled count and rewrites the display
projection in place. It clears the dirty marker before rewriting the display
values. The renderer reads the retained display projection directly. The
observed marker indicates a display update and supplies no completion receipt
or immutable observation for another caller.

| Existing path | Observed contract |
| --- | --- |
| Engine graph lifecycle | Changes active state and invokes processor lifecycle callbacks. No Scope-specific payload is returned. |
| Engine publication lock | Protects the published graph during rendering. Some host timing fields are updated before the callback tries the lock. The lock domain for all relevant Scope writers remains unverified. |
| `suspendProcessing` | A locked setter on the device pad-section component. |
| Graph and processing-error notifications | Carry their notification identifiers. No Scope capture fields were observed. |

The [earlier capture and display results](2026-11-26-scope-capture-display.md)
still apply to their tested settings and screenshots. Direct status and raw
waveform qualification require an observation tied to one current Scope
instance and its capture generation, format rate, frame, latch, valid count
and configured window. A producer-owned publication or a capture-preserving
read boundary remains open.
