# Public MCP Event Input read with Serum 2 FX

R533 ran the release MCP binary and bridge on Studio Pro 8.1.1 build
113103 arm64. One Music source in a stopped scratch song was routed to
one Xfer-signed Serum 2 FX VST3 2.1.5 insert on a separate Audio track.
The DSP identity reached from the Music source matched the exact Audio
insert UUID at every phase. The Audio receiver and Main were muted.

The stock Inspector independently marked Event Inputs 1, 2, 16 and the
restored 1. The public `track_event_input_get` operation returned the
same live port number and label twice at each stop. The separate Audio
receiver's name was refused with `host_rejected` and no result. The
public operation read the route. Test-only setup changed the source
port.

All 2,623 DSP rows, the bound FX identity, the timeline and mixer
matched after restoring Input 1. The trial made 124 requests in
136.088 seconds. The app and owner exited normally after about 146
seconds. No Save, MIDI or playback was issued. Scratch bytes, all 18
managed files and 10 affected profile files matched after cleanup.

[Serum 2 FX selected-port MIDI delivery](2026-11-26-serum2-fx-higher-channel-special-controls.md)
