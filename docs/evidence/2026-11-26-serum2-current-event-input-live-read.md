# Live script lookup of the selected Serum 2 Event Input

R530 checked the Xfer-signed Serum 2 instrument VST3 2.1.5 in Studio
Pro 8.1.1 build 113103 on macOS arm64. The named scratch song contained
one instrument track and one exact native Serum instance. Main was muted
at zero, the transport was stopped, and recording and monitoring were
off.

The stock Inspector independently showed Event Input 1, 2, 16 and the
restored 1 in that order. The private EditTask dispatched only the
three transitions on that track using the enumerated port object. At
each stop a separate read-only probe ran twice and both results agreed.

| Stock Inspector Event Input | Script `getDestinationChannel()` | Enumerated ports |
| --- | --- | --- |
| 1 | `null` | 16 |
| 2 | `null` | 16 |
| 16 | `null` | 16 |
| 1, restored | `null` | 16 |

Three tested `findParameter` names returned `null` at all four stops
(`port`, `destination`, and `output`). The eight probe responses
matched in full across the route changes. The tested script methods
therefore cannot report the current Event Input for this instrument.
The check did not assess other owner classes, a native port getter, or
same-model ambiguity.

All 2,623 controller rows, the exact instrument instance, mixer and
timeline matched after the restored route. The trial made 101 requests
in 73.424 seconds. The MCP owner and app both exited normally, about
85 seconds after launch. No MIDI events, playback, or Save were
issued. Scratch bytes matched the original, the temporary signed
bundle was retired, and all 18 managed files and 10 affected profile
files were restored and independently checked.

[Previously verified selected-port delivery](2026-11-26-serum2-selected-event-input.md)
