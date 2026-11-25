# Pedalboard vacant-slot value across save and restart

R338 used a single owned scratch in Studio Pro 8.1.1 build 113103 on macOS
arm64. The diagnostic native candidate was the same occupied/empty admission
used in [R317](2026-11-25-pedalboard-empty-slot.md). Compressor began in slot
1 with distinguishable gain and slot 2 was empty. A guarded exchange moved
the complete Compressor payload into slot 2. The inverse restored the order
visibly and preserved the mixer and the separate Phase Meter instance.

| Observation | `controls.active[stomp2]` raw | Relative to the initial 33-row read |
| --- | --- | --- |
| Before exchange | 0 | Baseline |
| After reverse exchange | 10000 | This is the only differing row. The component fingerprint matches the original populated capture. |
| After one save, in the same session | 10000 | The one differing row remains. |
| After complete app restart | 0 | All 33 typed rows match the populated baseline. |

The owned saved song's native `.dsppreset` contains Compressor and its gain
in slot 1. Slot 2 contains only the zero class ID. The fresh process read the
same saved bytes, observed the same peer parameters, and refused a read with
the old document reference. The vacant-slot activity residue is therefore
observed in the original live session and absent from the saved vacant slot.
The public adjacent-swap admission remains limited to occupied slots.

The two hosts were open for 104.194 and 29.546 seconds. The trial recorded 90
requests and the reopen 17. The saved scratch was retained with no second
save, and all 18 managed files and ten changed profile files were restored.
