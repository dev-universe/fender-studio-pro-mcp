# Mustang Native processor state in Studio Pro 8.1.1

R544 exercised the public `plugin_state` action `read_mustang_state`
against one exact Mustang Native stereo insert in Studio Pro 8.1.1
Build 113103 on macOS arm64. The test used a write-guarded copy of a
blank test song, a stopped transport, a silent Main, and one MCP
owner at a time. The tested host is the local 8.1.1 installation.
The core and native extension were built from `bbba6e6`.

| Check | Observed result |
| --- | --- |
| Exact native identity | The DSP inspection and all four observations agreed on the Mustang Native AudioEffect class and instance. |
| Serialized processor projection | Four observations across two owners agreed on 26 ordered slots, three populated processors, and 20 numeric settings. The model and settings digests matched on every read. |
| Independent DSP read | All 39 parameter rows, including typed values and metadata, matched before and after. Track, event and mixer state matched. |
| Raw serialization | Four observations and two ordinary captures were each 3,518 bytes. All six raw capture digests differed. Each raw digest describes one capture. Both ordinary capture tokens were released. |
| Refusals | Rumble Native with the Mustang action, the wrong instance UUID, a missing document, and a stale document were refused without returning processor data or a token. |

The first harness stopped at a full-response equality assertion
because the two raw capture hashes differed. A fresh MCP owner in the
same still-open test song verified the archived reads and confirmed
the bounded projection, all 39 DSP rows, and the four refusals.
No fixture edits were repeated. The two owners handled 23 requests
in 2.963 seconds and 18 requests in 2.411 seconds, respectively.

The owned song closed without Save and its disk bytes matched the
original. The host ran for 93.891 seconds and exited normally.
All 18 managed installation files and 11 changed profile files
were restored and rehashed. The test establishes the direct
serialized processor read, with model selection, audio processing
and persistence through Save outside its verified scope.
