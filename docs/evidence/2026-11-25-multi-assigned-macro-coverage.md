# Multi Instrument assigned macro coverage

R398 reopened the saved two-Mai-Tai fixture in Studio Pro 8.1.1
build 113103 on macOS arm64. The slots matched their saved identities,
and each current Device was bound and read separately. The selected
Music track had no arrangement events or monitoring. Transport was
stopped and Main was at zero.

| Root macro group | Exact changed child targets |
| --- | --- |
| Spread 1 and 2, `float-5` and `float-6` | `osc1.stereowidth` on the selected child, one slot per macro |
| Vol 1 and 2, `float-7` and `float-8` | `masterGain.gain` on the selected child, one slot per macro |
| Filter and Noise, `bool-1` and `bool-2` | `filter.active` and `noise.active` respectively on both children |
| Character 1 and 2, `bool-3` and `bool-4` | `character1.active` on the corresponding child |
| Rnd Phase 1 and 2, `bool-5` and `bool-6` | `osc1.random` on the corresponding child |
| Synth 2, `bool-8` | The second child's inverted `bypass` |
| `pad-x-1`, `pad-y-1`, and `pad-x-2` | Both children's `filter.cutoff`, `filter.resonance`, and `character1.eqcolour` respectively |

Each case changed only the bound parent macro and the listed child
rows. Two independent reads established the changed state, and two
more matched all 716 original rows after restoration. Together with
the earlier Cutoff, Resonance, Filter Env, Synth 1 and Amount trials,
19 of the 20 saved macro roles have evidence for live change and
recovery. Filter Decay remains a separate fixture because its initial
parent and both child values differ. No MIDI event or audio
measurement was part of R398.

The owner used 790 requests in 59.934 seconds and the app ran for
112.570 seconds. Both exited normally. The unmodified scratch bytes
and all 18 managed files and nine profile files were verified after
the scratch closed without saving.
