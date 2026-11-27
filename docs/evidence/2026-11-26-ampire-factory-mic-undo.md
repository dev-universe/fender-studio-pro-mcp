# Ampire factory microphone mix after one editor Undo

Studio Pro 8.1.1 Build 113103 on macOS arm64 opened a blank owned
scratch song. One stock Ampire insert was bound to a muted empty
track, with transport stopped and Main muted at zero. The native
editor showed MCM 800, 4x12 M65, Link on, and 80%, 10%, 10%.

One upward drag on Mic B changed the three linked displayed levels to
61%, 31%, 8%. An independent typed comparison found exactly the three
microphone mix rows changed. The Edit menu's top Undo entry named the
observed Mic B Mix change. Applying that one entry returned the visible
levels to 80%, 10%, 10%, with Link still on.

| Native row | Original raw | After one drag | After one Undo |
| --- | ---: | ---: | ---: |
| Mic A Mix | 7957.139611244202 | 6132.3487758636475 | 7957.139015197754 |
| Mic B Mix | 1016.8377310037613 | 3076.923191547394 | 1016.8377310037613 |
| Mic C Mix | 1026.0231792926788 | 790.7278835773468 | 1026.0231047868729 |

A and C differed from the starting bits. The component fingerprint
also differed. The other stable settings, exact insert, mixer and
empty timeline matched. This editor path restored the displayed mix.
Exact factory-state recovery remains open.

The run took 60 MCP requests in 112.004 seconds. Three state tokens were
released. The app exited normally after 150.141 seconds. The scratch
closed without saving and retained its original digest. Eighteen managed
installation files and ten app profile files were restored and verified.
