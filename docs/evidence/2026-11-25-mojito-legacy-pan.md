# Mojito legacy preset pan in Studio Pro 8.1.1

Two stock Mojito presets contain `pan=0` in their legacy `ParameterData`.
The same class's current 37-row DSP provider and a fresh saved instrument
state have no `pan` entry. Mixer pan is a separate host control.

REA with Ghidra and an independent ARM64 disassembly traced the current
loader in the build 113103 `synth` image. The arm64 Mach-O UUID is
`1DBFCCF8-40E1-3721-9D49-E0CC9A7E2057` and the thin SHA-256 is
`1b69b41a5e6f6f4d179a53a20c50301d25824e2b215fb56a04976324c1980c32`.
The `ParameterData` restore path iterates the registered descriptors and
looks up their canonical or compatibility names. Mojito's descriptor
registration has no `pan`. The older-version migration slot on the Mojito
algorithm resolves to the default function that returns zero without reading
attributes. Craft Anything CLI 0.6.1 in Deep mode independently decoded that
short routine. The `pan` string found elsewhere is registered with shared
channel properties and does not add a Mojito parameter.

R335 opened three owned scratch copies in separate sessions. The copies
differed only in the embedded Mojito component preset, with a control copy
containing each of these attribute changes.

| Fixture | Embedded change | Studio Pro 8.1.1 result |
| --- | --- | --- |
| Baseline | `pan=0` | 37 stable typed rows and centered mixer pan 0.5. |
| Comparison | `pan=1` | All 37 rows and full mixer replies match the baseline exactly. |
| Positive control | `pan=1` plus `volume=0.25` | Only the DSP `volume` row changes, from 10000 to 2500. The mixer reply matches exactly. |

In each case the provider returned two complete, matching reads. A separate
name lookup returned `volume` and reported `pan` missing. This establishes
that the tested legacy attribute has no effect in the current Mojito
parameter restoration path. The observations cover host state, with no
physical or rendered stereo-audio measurement.

The app was open for 96.970 seconds across the three cases and the MCP
recorded 30 requests. Each scratch closed without saving, file hashes
matched, and the managed installation and profile were restored.
