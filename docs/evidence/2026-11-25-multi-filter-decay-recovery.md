# Exact DSP recovery of the Multi Instrument Filter Decay fixture

R400 and R401 used Studio Pro 8.1.1 build 113103 on macOS arm64 and
the saved two-Mai-Tai fixture. The assigned Filter Decay macro
changed both `adsr2.decay` values. The initial state token restored
the original Filter Decay macro and all 566 child rows, including
both exact original Decay value bits. It also replaced both live
child Devices. The original logical slot IDs and parent stayed
unchanged. The only remaining DSP difference was the unrelated
Filter Env pilot at the root.

R401 followed capsule restoration with the established Filter
Env repair. It wrote the original root Filter Env bits and
then restored each child's amount through
`plugin_multi_child_filter_env_set`. The old pre-capsule child
identity was refused before a write. Fresh child bindings retained
their expected before and after bits, and independent reads after
each call confirmed the root and peer values. The final two reads
matched all 150 parent and 566 child DSP rows.

The final capsule fingerprint differed from the initial capture
even after the 716 exposed rows matched. This establishes exact
DSP recovery with newly bound Devices. The complete serialized
payload and unchanged live Device identities remain outside
that recovery claim.

R400 used 80 requests in 7.103 seconds and ran for 79.560
seconds. R401 used 176 requests in 14.166 seconds and ran for
66.508 seconds. The four and five respective state tokens were
released. Both scratch copies stayed byte-identical, and the
managed installation and nine profile files were restored.
