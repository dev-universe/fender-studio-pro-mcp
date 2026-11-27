# Multi Instrument root macro and child restoration can differ

R392 reopened a fresh copy of the same two-Mai-Tai scratch fixture in
Studio Pro 8.1.1 build 113103 on macOS arm64. The exact saved slot IDs
bound to two independently read live child Devices. Main was at zero,
transport was stopped, and no MIDI or audio signal was sent.

The first case, `float-2` Resonance, changed `filter.resonance` on both
Mai Tai instances. Restoring the macro restored all 150 root rows and
both 283-row child providers. The next case, `float-3` Filter Env, changed
only the declared `modulationTable.cell1.amount` row on each child.

| Point | Root Filter Env, raw | Each child amount, raw |
| --- | ---: | ---: |
| Saved baseline | 5449.993014335632 | 5003.644824028015 |
| Bound write | 5000 | 5000 |
| Root restoration | 5449.993014335632 | 5449.993014335632 |

The root restoration returned the exact original root bits. The child
values converged to the macro's pilot value, which differed from their
own original bits. Reading the root alone would miss the remaining
change. The trial stopped there and issued no writes to later macros.

The 127 MCP requests took 10.051 seconds. The host exited normally
after 90.543 seconds. The scratch was discarded without saving and was
byte-identical. All 18 managed files and nine profile files were
restored. The evidence establishes the differing live child state
after a root-only restore for this fixture.
