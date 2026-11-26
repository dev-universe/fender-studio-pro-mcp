# Ampire global Wah alias on the Wah Wah stomp

On Studio Pro 8.1.1 build 113103, macOS arm64, R359 bound a single
Ampire AudioEffect to an empty muted stereo track with silent Main.
The exact document, bridge session, insert slot and native instance
were checked. Transport, monitor and record arm remained off.

The model list reported Wah Wah at index 15. After selecting it in
previously vacant slot 2, a separate native read and the Ampire editor
showed the intended model. Its `stomp2.pedal` and the global
`controls.wah` were both writable and began at raw 0. The other slots
and mixer stayed in place.

Sending normalized 0.625 to only the alias produced raw 6250 in both
independently read rows. An exact-bit write returned them to 0. No other
stable row changed. Two fresh reads of all 93 non-meter settings and the
full serialized fingerprint matched the prepared Wah fixture at the
end. The two issued capsules were released. The test did not measure
audio or save a preset.

The MCP owner used 67 requests in 30.463 seconds. The host exited
normally after 61.780 seconds. The scratch remained byte-identical and
all 18 managed files and 10 touched app-profile files were restored.
