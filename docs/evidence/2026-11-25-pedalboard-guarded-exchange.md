# Fingerprint-guarded Pedalboard exchanges

On the exact Pedalboard insert, first take a fresh `plugin_state` capture.
Use its top-level `sha256` as `expected_rack_sha256` with the exact
instance, current model display labels and two one-based pedal slots.
The vacant model label is the literal `" - "`. A verified serialized
exchange receipt means the entire captured component matched the
expected permutation. Take a second capture to confirm the eight
payload fingerprints independently, then release both tokens.

R343 validated the public MCP path on Studio Pro 8.1.1 build 113103,
macOS arm64. Three separate songs and app runs each rejected a validly
formatted but wrong rack fingerprint before any change. The same request
with the matching fingerprint exchanged the selected payloads. A second
guarded call using a fresh capture reversed the exchange. Phase Meter,
the mixer and the empty timeline matched throughout.

| Case | Verified exchange | DSP state after inverse | Requests | App seconds |
| --- | --- | --- | ---: | ---: |
| Vacant | Compressor moved from slot 1 to empty slot 2, then returned | The vacant-slot active alias still read 10000 | 82 | 56.394 |
| Same model | Two Compressors exchanged Gain and active settings | Four slot-specific Gain/active rows still reported the prior payload | 88 | 62.165 |
| Nonadjacent | Compressor and Big Fuzz exchanged slots 1 and 3; Reverb stayed in slot 2 with its original fingerprint | All setting rows matched after the inverse | 86 | 72.247 |

The editor and independent component captures confirmed both directions
in each case. The original complete component SHA-256 returned after
every inverse. There was no state reload. In the identical-model case
the slot-specific controller rows remained one exchange behind in both
directions. Two read-only output peak rows were excluded from the
nonadjacent setting comparison.

The three successful trials used 256 requests in 94.782 seconds, with
190.806 total app seconds. Three preparatory trials exposed a test
harness assumption about error flags, controller readback after the
inverse, or output meter stability. All six scratch documents closed
without saving and preserved their original bytes. All capture tokens
in the successful trials were released. The three app profiles and the
normal 18-file installation were restored.

The receipt remains `applied_unverified` for the wider effects of the
edit. It does not establish audio, automation, Undo behavior in these
new cases, or durable project state. The prior capture and exchange
contracts for other plug-ins and the existing adjacent occupied
Pedalboard call are exercised by the server and bridge tests.
