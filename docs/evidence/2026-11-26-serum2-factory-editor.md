# Opening a Serum Factory catalog result in Studio Pro

R499 used macOS arm64 and Studio Pro 8.1.1 build 113103 with the
Xfer-signed Serum 2 and Serum 2 FX 2.1.5 VST3. The candidate Rust server
ran as one MCP owner in a stopped, muted scratch song. Each variant's
`serum2_factory_presets_list` result matched an independent read of the
file header. Both selected files reported product version 2.0.13,
schema 5.

For each variant, the exact returned file was opened using **Menu →
Load Preset** in the corresponding Serum editor. Each editor showed
the title from its catalog entry. Subsequent MCP reads used the same
document and instance identifiers.

| Variant | DSP rows read | Changed values | Additional metadata-only changes |
| --- | ---: | ---: | ---: |
| Serum 2 | 2,623 | 61 | 23 |
| Serum 2 FX | 2,623 | 26 | 16 |

All 2,623 peer rows matched their preceding read after the other
instance changed. The mixer, sends, empty timeline, instance identities
and source file fingerprints matched throughout. The FX preset enabled
its own Note Latch. The test sent no MIDI and made no transport
requests.

The trial issued 203 MCP requests in 177.941 seconds. The app ran for
226.436 seconds and closed without saving. Both processes exited
normally. The temporary signed bundle was removed from the scan path
and 18 managed files and ten profile files were restored and verified.
