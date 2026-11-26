# Serum 2 arrangement playback in Studio Pro

R449C used the official Xfer-signed Serum 2 VST3 2.1.5 in demo mode on
Studio Pro 8.1.1 build 113103 and macOS arm64. The MCP loaded the instrument
and applied the [processor-only carrier](2026-11-25-serum2-mcp-vst3-preset.md)
to its exact current instance. A separate Rumble Native instance supplied
read-only stereo input peaks from the full 39-row DSP provider.

`tracks_edit` created one unmuted part spanning 0–8 seconds at 120 BPM.
The part contained one MIDI note with pitch 60 and velocity 64, from
2.000 to 5.000 seconds. `tracks_list` and `notes_read` independently
confirmed the part and note. The editor view showed the single part and
the stopped playhead before playback. Loop, metronome click, precount,
preroll, record arm and monitoring were off. The track had no external
input or sends, and Main was muted with volume zero.

| Observation | Result |
| --- | --- |
| Before Play and before note onset | Three initial quiet reads and five quiet reads while the playhead preceded the note |
| During the note | Seven eligible stereo samples, bracketed by independent transport reads, with raw medians of 3173.3617 left and 3173.3599 right |
| Transport | One explicit Play and one Stop, with stopped state independently read at 5.715 seconds of arrangement position |
| After Stop | Three consecutive quiet stereo reads at or below raw 0.0001, confirmed 8.109 seconds after independent Stop verification |
| Final readback | The same part and note, exact route and instance bindings, and all 2,623 post-apply DSP rows matched |

R449 stopped before creating the part when the metronome response reported
an unavailable preroll field. The independent transport state supplied that
field in R449B and R449C. R449B confirmed the exact part and note but stopped
before Play because an inspection mode was reserved for empty timelines.
R449C used that mode in preflight, then independently verified focused
instrument state, exact native identity, populated arrangement and routing.

The successful owner made 300 requests in 64.074 seconds. Studio Pro and
the owner exited normally. The scratch song was closed without saving, the
temporary VST3 was retired, and all 18 managed installation files and ten
profile files were restored. Physical MIDI and audio output remain separate
tests. The reported quiet delay describes meter recovery.
