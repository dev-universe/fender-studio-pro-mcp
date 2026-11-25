# Serum 2 and Serum 2 FX VST3 MIDI controller mapping

R453 used the official Xfer-signed Serum 2 2.1.5 arm64 VST3. Two isolated
hosts connected the exact instrument and FX components to their respective
edit controllers and provided each controller its own component state. Both
connected controllers answered the SDK's `IMidiMapping` queries for all 130
controller numbers on input event bus 0, MIDI channel 0. The MIDI channel
number is zero-based. The two mappings agreed and every returned ID appeared
once in the corresponding 2,623-entry parameter catalogue.
[Steinberg IMidiMapping reference](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IMidiMapping.html)

| MIDI controller | VST3 parameter ID | Studio Pro parameter name |
| --- | ---: | --- |
| CC 0 | 500000000 | `vstp500000000` |
| CC 1 | 516000000 | `vstp516000000` |
| CC 64 | 500000064 | `vstp500000064` |
| Channel pressure, controller 128 | 500000128 | `vstp500000128` |
| Pitch bend, controller 129 | 516000001 | `vstp516000001` |

REA and Ghidra resolved the connected interface method in the same signed
binary and independently corroborated the two special IDs and the ordinary
channel-0 addition. Craft Anything independently matched the method's full
116-byte range. Earlier Studio Pro runs R449C and R445FXC independently
exposed all 130 exact names on their own Studio Pro 8.1.1 DSP provider. The
host's numeric row `tag` was one greater than the VST3 ID in each of these
rows. Send the returned parameter name when resolving a Studio Pro row.

The channel-0 lookup is a reference for this build and architecture. A wider
isolated query ended before completion. Other channels, Note Expression
semantics, MIDI 2.0 and controller delivery remain outside this evidence.
The two completed reference probes exited normally. No DAW, hardware
output, transport operation or parameter mutation was involved in this
offline lookup. The real-DAW evidence here concerns parameter name and
owner resolution.

The release MCP server returned all 260 observed mappings over stdio on an
isolated profile with Studio Pro closed. Five invalid requests were refused,
including a controller outside the reference range and an extra channel.
