# Mojito MIDI reception in Studio Pro 8.1.1

R334B used a single owned scratch in Studio Pro 8.1.1 build 113103 on
macOS arm64. A temporary Core MIDI source sent MIDI 1.0 on channel 1 through
one verified generic keyboard device to the monitored Mojito track. A
separate Core MIDI loopback first confirmed the six message payloads. During
the host trial the transport stayed stopped, the track remained unarmed, and
Main remained at zero. Every MCP read used the exact Mojito instance of class
`19C6BEAB-1A48-4338-BB04-14A9AC1FCC05`.

| Message pair | Mojito DSP parameter | Independent typed observations |
| --- | --- | --- |
| Channel pressure 96 → 0 | `afterTouch`, tag 16777345 | 0 → 7559.0550899505615 → 0. Sixteen active reads and two fresh zero reads. |
| CC 1 value 96 → 0 | `modWheel`, tag 16777217 | 0 → 7559.0550899505615 → 0. Sixteen active reads and two fresh zero reads. |
| Pitch bend 12288 → 8192 | `pitchBend`, tag 16777344 | 5000 → 7500.457763671875 → 5000.30517578125. Sixteen active reads and 43 reads at the post-message value. |

The other two roles stayed at their baseline in each episode. The received
pitch-bend values are consistent with mapping the unsigned 14-bit value to
0–10000 using 16383 as the denominator and storing a float32. The host's
parameter display showed `+0.01` after the center message. Two complete
reads after the sender closed, and two more after the track input and
monitoring were restored, found `pitchBend` as the only writable difference.
One exact typed MCP write restored the original 5000. Two independent
complete provider reads confirmed that all 35 writable values, raw bits,
and their normalized representations matched the original values.

The initial attempt created its source after the DAW started and stopped
before sending messages when that source could not be selected. R334B
registered the source before launch. The host was open for 197.569 seconds.
The primary owner recorded 119 requests and the three follow-up owners
recorded 34 requests. The scratch closed without a save, its original bytes
matched, and 18 managed files and ten profile files were restored. The
virtual source exited and left no additional profile cache member.

This establishes software MIDI delivery and exact setting restoration in
this route. Mojito's legacy stored `pan` field and the physical MIDI input
path remain outside this observation.
