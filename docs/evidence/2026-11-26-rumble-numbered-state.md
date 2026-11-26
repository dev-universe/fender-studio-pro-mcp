# Rumble Native processor state on numbered inserts

R487–R490 used Studio Pro 8.1.1 build 113103 on macOS arm64.
The scratch had Tone Generator followed by three Rumble Native
instances. Main was muted at zero, the test channel was at zero,
and the transport stayed stopped.

The plugin class and all three native instance IDs were available in
R487. Studio Pro supplied the native object labels `Rumble Native`,
`Rumble Native 2`, and `Rumble Native 3`. The state read for the
middle instance nevertheless failed after its accepted preset recall.
R488 changed native and JavaScript validation and still failed to read
the numbered peer. R489's private native trace reached parsing for
the first and third inserts. For both, the saved component still used
the canonical `Rumble Native` class name. The MCP runtime had an
additional check for the canonical name in the native object label.

R490 used product source `59ac9e0`. The same `plugin_state` request
with `action: "read_rumble_state"` preserved all three native labels
and returned 26 processor slots on each exact instance. A request for
the middle slot with the upstream peer's ID returned `not_applied`
and supplied no processor data. Native and server-side request checks
still bind reads to the current document and the requested instance.
The serialized component class, native class ID, processor layout,
stage-retained identity, and response digest checks remained in effect.
The state read remained read-only and returned no token.

R490 made 186 requests in 19.992 seconds and exited normally after
92.592 seconds. A separate first-phase signal sequence was bounded
to one Tone Generator On and Off. Its final state reads had the same
model and settings hashes. Four read-only peak rows were still
decaying below 0.00002, so the exact full-DSP comparison stopped that
signal test. The numbered state and peer refusal had already been
observed. The scratch closed without saving and the original 18
managed installation files and ten app-profile files were restored.
