# Public MCP Event Input setter for Serum 2 and Serum 2 FX

R536 and R537 used the release MCP build on Studio Pro 8.1.1 build
113103 arm64, with Xfer-signed Serum 2 VST3 2.1.5. The named scratch
songs were stopped with Main muted at zero. The public setter
checked the native document, exact selected Music track and Serum
instance, current port, and matching owner of the replacement port.
Each accepted receipt confirmed one dispatch followed by independent
MCP route readback and the same Serum instance.

## Two Serum 2 instruments with Mai Tai

R536 used two separate Music tracks with distinct Serum 2 instance
UUIDs and a stock Mai Tai peer. Independent reads around each public
mutation showed `1/1 → 2/1 → 2/16 → 1/16 → 1/1`. Three requests
were refused before dispatch for a wrong instance, stale port, or
wrong variant. Both Serum DSP snapshots matched across 5,246 rows.
Mai Tai's native identity matched after the changes. The trial made
252 requests in 49.848 seconds.

## Serum 2 FX

R537 used one separate Music source routed to one exact Serum 2 FX
insert on a muted Audio receiver. A fixture-only setup attached the
Music source at Event Input 1. The public setter and eight independent
reads then agreed on `1 → 2 → 16 → 1`. A stale input was refused
before dispatch. The same FX UUID remained reachable from both the
Music source and the Audio insert, and all 2,623 DSP rows matched.
The trial made 125 requests in 37.805 seconds.

Both owners and hosts exited normally. The scratch copies stayed
byte-identical. All 18 managed files and 10 affected profile files
per trial were verified after restoration. These trials covered live
route selection and readback. MIDI delivery and saved state have their
own evidence.

[Instrument MIDI delivery](2026-11-26-serum2-higher-channel-special-controls.md)
[FX MIDI delivery](2026-11-26-serum2-fx-higher-channel-special-controls.md)
