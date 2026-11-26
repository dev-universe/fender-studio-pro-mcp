# Ampire standard and Strobe tuner controls

R354 exercised an exact Ampire insert in Studio Pro 8.1.1 build 113103 on
macOS arm64. The track was muted and empty, Main silent and the transport
stopped. The exact document, session and native instance were checked.

Opening the tuner panel through its own toolbar changed only
`opt.tunerbypass`. The standard display and the Strobe switch in its
lower position were visible. The typed MCP operation changed
`tuner.gaugeMode` from 0 to 1. Two independent reads showed only the
gauge mode changed, and the visible Strobe switch moved up. Returning it
to 0 restored the lower position and the typed baseline of the open
panel. The test then closed the tuner through the same toolbar control.

After closing, the full serialized component hash and all 90 non-meter
settings matched the original. Mixer, timeline and insert state were
preserved. The tuner displayed 440 Hz calibration. No pitched input or
audio was sent, so the test covers the mode switch and panel lifecycle.

The owner used 80 requests in 70.998 seconds and exited cleanly. The
app ran 101.637 seconds. The scratch song was discarded and remained
byte-identical. All 18 managed files and 10 changed profile files were
restored and verified.
