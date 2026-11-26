# Ampire retained microphone levels after FAT recovery

R482 repeated the [FAT recovery](2026-11-26-ampire-fat-linked-recovery.md)
on Studio Pro 8.1.1 build 113103, macOS arm64, with runtime `ccfaec0`.
The scratch had one Ampire insert on a muted empty track. Main was
muted at zero and transport, monitoring and record arm were off.

The FAT 50%, 75% and 50% cycle and capsule load reproduced the earlier
2,297 → 2,294-byte change. The four microphone recovery writes restored
89 stable rows and the original SHA-256 of the 2,297-byte capsule with
Link on. The trial then made exactly one additional Link-off write.
The write readback matched. Two independent complete reads of the same
instance agreed, with the comparison excluding only the tuner meter.
All rows outside Link and the three microphone gains matched.

| Mic | Factory first Link off (R351) | Link off after FAT recovery (R482) | Float32 ULP delta |
| --- | --- | --- | ---: |
| A | `3F4BB3E9` | `3F4BB3E8` | −1 |
| B | `3DD03F95` | `3DD03F94` | −1 |
| C | `3DD2212A` | `3DD2212A` | 0 |

The values are normalized Float32 bit patterns. The reference is the
[first Link-off read](2026-11-25-ampire-unlinked-mic.md) on the original
MCM 800 and 4x12 M65 state. R482 reproduced that reference's linked
baseline before the FAT cycle. A matching linked capsule and readback
can therefore coexist with a different next Link-off read in this
fixture. The observation ends at Link off.

The single owner made 135 requests in 12.437 seconds. The app exited
normally after 38.308 seconds. The owned file was unchanged, with zero
Save, playback or MIDI dispatches. All 18 managed installation files and
10 changed app profile files were restored and rechecked.
