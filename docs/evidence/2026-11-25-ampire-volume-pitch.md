# Ampire Volume and Pitch alias observations across the installed stomp list

R360 used Studio Pro 8.1.1 build 113103 on macOS arm64. One exact
Ampire instance was on an empty muted stereo track with the transport
stopped and Main silent. The MCP read the complete 16-entry stomp list.
The trial observed vacant slot 2, then selected each of the 15 actual
models in that same slot.

For all 16 states, two separate reads returned `controls.volume`
and `controls.pitch` as Float values with raw 0 and empty display text.
The original-chain query for each alias returned only its own entry
with a self-reference. The same original-chain result also occurred for
the positively evidenced Speed and Wah mappings. The lineage query alone
cannot distinguish a mapped alias. This pass found no different Volume
or Pitch observation in this installed catalog. Neither alias was
written and no processing claim follows from these reads.

Selecting the vacant item again returned the baseline full component
fingerprint and left the slot without a native stomp child. Two typed
rows differed from their initial observation. The writable empty-slot
activation alias read raw 10000 instead of 0. The read-only tuner
difference read raw 0 instead of 5000. Mixer, timeline and insert chain
were stable.

The owner used 234 requests in 20.732 seconds and exited normally.
The host ran 64.493 seconds. No song was saved, the scratch file
remained identical, and 18 managed files and 10 touched profile files
were restored and verified.
