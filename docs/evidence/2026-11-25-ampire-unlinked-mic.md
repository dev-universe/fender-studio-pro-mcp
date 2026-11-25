# Ampire independent mic levels and relink precision

Studio Pro 8.1.1 build 113103, macOS arm64. R351 used the MCM 800 and 4x12
M65 on an exact Ampire insert on a muted empty track. Main was silent,
the transport stopped, and monitor and record arm were off.

- Disabling Mic Mix Link changed only `cab.micmixlink` in two independent
  reads of all 90 non-meter values.
- Setting Mic B to normalized 0.25 changed only that row, to raw 2500.
  A separate exact-bit write restored it. Two complete reads matched
  the unlinked baseline.
- Setting Mic C to normalized 0.375 changed only that row, to raw 3750.
  A separate exact-bit write restored it as well.
- Re-enabling Link recovered the Boolean but altered Mic B and Mic C:
  each raw value decreased by 0.0002980232238769531. Mic A and the other
  typed rows matched the original. The whole serialized fingerprint
  differed. No further changes followed this result.

| Row | Linked original | After unlink, exact writes, and relink |
| --- | ---: | ---: |
| Mic A | 7957.139611244202 | 7957.139611244202 |
| Mic B | 1016.8377310037613 | 1016.8374329805374 |
| Mic C | 1026.0231792926788 | 1026.022881269455 |

The mixer, timeline, and insert list were unchanged. The two capsules
were released without any load. The 90 requests took 8.552 seconds,
and the app exited normally after 46.533 seconds. The original scratch
file was unchanged, and 18 managed and 10 touched app-profile files
were restored. Physical output was outside this check.
