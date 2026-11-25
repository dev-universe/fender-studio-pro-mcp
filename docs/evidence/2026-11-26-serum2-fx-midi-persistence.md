# Saved Serum 2 FX MIDI destination in Studio Pro 8.1.1

R467 used Studio Pro 8.1.1 build 113103 on macOS arm64 and the
Apple-notarized Xfer Serum 2 2.1.5 VST3. A private scratch had one
instrument track and two separate Audio channels for same-model Serum 2
FX inserts. The transport stayed stopped and Main was muted at zero.

The first Save kept the instrument track's original Serum 2 output, with
one Serum 2 FX already loaded. The stock Out menu showed that instrument
and one eligible FX. After choosing the FX, a second copy was loaded on
the peer Audio channel. Studio Pro listed `Serum 2 FX` and `Serum 2 FX 2`
separately and kept the first selected. After the second Save and a
fresh open, the original FX was still checked in the stock menu.

The first archive connected the Music channel to its instrument's saved
device UUID. The second retained the same source channel and instrument
UUIDs and encoded `{device UUID}/MusicInput00` for the first FX. The
UUID matched the first insert's `deviceData` unique ID and differed from
both its rack slot ID and the peer's two IDs. This result was read from
the exact file again after reopen. In the live app, `plugin_inspect`
resolved the Music track's active plug-in to the same exact native
instance as the target insert. The same-model peer had its own distinct
native instance. A fresh MCP owner confirmed those relationships again
under the reopened document's new lease.

Two initial operator checks required adjustments to match observed host
behavior. Inspecting the Music track after changing its output returned
the FX instead of the former synth. The peer rack used the generated
`Serum 2 FX 2` name. The trial retained the first Save and the original
live session, then used fresh guarded readers without reapplying the
route or reinserting the peer. The four owners made 50, 24, 16, and 16
requests respectively. The final guarded phase took 75.439 seconds.

There were two Saves, both to the privately owned scratch. No MIDI or
transport actions were issued. Studio Pro and the MCP owners exited
normally. The temporary signed scanner bundle was retired, and 18
managed files and 10 app profile files were restored and verified.

The saved reader uses the Music track's channel UUID and the unique
matching saved plug-in device UUID. An ambiguous, unresolved, or
unsupported reference yields no destination. This result covers the
persisted output and the observed live identity. Use a fresh live
read after editing the routing in Studio Pro.
