# Multi Instrument Filter Decay child precision

R399 reopened a copy of the saved two-Mai-Tai fixture in Studio Pro
8.1.1 build 113103 on macOS arm64. The exact parent and both child
slots were bound. Main was at zero and transport was stopped.

Writing the Filter Decay macro `float-4` from 750.0004023313522 to
5000 changed only the parent's macro and both children's
`adsr2.decay` rows. Both child values read 400.00002831220627. The
return write restored all 150 original parent rows. Two separate
child reads agreed that only the two Decay values differed from
their original bits.

| State | Each child Decay raw value | Float64 bits |
| --- | ---: | --- |
| Initial | 0.059696290009014774 | `3FAE90831A900000` |
| After restoring the parent macro | 0.05969631729385583 | `3FAE908404F00000` |

The two typed child rows are at index 40, tag 41, with Float carrier
and nominal raw range 0–10000. This case establishes the change
route and the precise residual of a root-only round trip. The
scratch was discarded without saving. Its bytes, 18 managed files,
and nine profile files matched after cleanup.

The owner completed 85 requests in 7.328 seconds and the app ran
69.482 seconds. Both exited normally.
