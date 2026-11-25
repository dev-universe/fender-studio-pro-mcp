# Multi Instrument child DSP readback

R330 used the `48b2ca9` candidate in Studio Pro 8.1.1 build 113103 on macOS arm64. It bound one stopped scratch song, a selected unarmed Music track, the Multi Instrument parent, and the current slot and Mai Tai devices. Main was silent. `plugin_multi_child_parameters` used direct child DSP and returned a read-only paged report.

| Factory index | Visible graph | Independent DSP projection |
| --- | --- | --- |
| 47 | One connected Mai Tai | All 283 rows in `Inst01`, read twice |
| 48 | Two visible Mai Tai branches | All 283 rows in each of `Inst01` and `Inst02`, read twice per child |
| 47 | The original one-child graph | All 283 child rows and all 150 root parameter rows matched the original exact typed projections |

The two child devices under preset 48 had distinct live UUIDs. `noise.color` raw values were approximately 5050 and 5000 in the first and second slot. `voiceAmp.velo-amount` values were approximately 5000 and 5050 respectively. These are the native DSP units. Both separate reads agreed, including the reported value encodings and metadata. The starter page returned one row and advertised 283 on the same exact pointer-derived binding.

Separate requests with a wrong parent UUID, slot UUID or child UUID, and a stale document were refused. The existing controller observation also refused a wrong instance. The subsequent factory restore retained ordered slot identities and stable typed values through token expiration. Opaque state hashes varied across repeated captures while the scoped DSP state was stable. Native event successors, routing under active input, and other child types remain outside this result.

The trial used 174 requests in 117.724 seconds. Studio Pro exited normally after 181.414 seconds. The scratch closed without saving. The original 18 managed files and modes, ten application profile files and the protected scratch were verified after restoration.
