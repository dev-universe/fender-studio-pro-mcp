# Impact sample and settings after reopening a saved song

R406 used Studio Pro 8.1.1 Build 113103 on macOS arm64. The owned song
had one selected Impact track, an empty timeline, stopped transport and
a muted Main at zero. An original 14-second, 48 kHz, 440 Hz WAV was
loaded onto bank A, C1. The editor displayed frames 0–672,000 and the
Normal trigger. The loaded sample changed the component fingerprint
while the 4,262 exposed DSP rows stayed exact.

A bound MCP request wrote the adjacent empty pad's
`pad[1].filter.cutoff` from raw 10,000 to 2,500. Two complete reads
witnessed only that row change. Studio Pro saved the named scratch
once. Readback matched all 4,262 edited DSP rows and the Impact
component fingerprint matched before and immediately after Save.

A new Studio Pro process opened the exact saved file. The new MCP
session and document rejected a request carrying the old document.
The Impact device retained its saved UUID and was rebound in the new
document. The editor showed the original sample label and frame
bounds on C1, with the Normal trigger still selected. Two complete
DSP reads matched the saved 4,262 rows, including the C♯1 cutoff
marker. Independent recaptures agreed with each other and matched
the saved component's 36,042-byte fingerprint.

The saved component contained the external WAV reference. The file
remained available at the same location throughout the test. The
song archive did not contain a matching WAV member, so media
portability between machines has separate acceptance criteria.
This run checked the reopened state and editor; the internal note
hold and release were exercised in the separate R405B trial.

The Save phase completed 310 MCP requests in 120.242 seconds and
the first app ran for 199.890 seconds. Reopen completed 155 requests
in 56.112 seconds and the second app ran for 100.545 seconds. Six
component tokens were released. The reopened scratch closed without
a second save, retaining its verified file digest. The source fixture,
normal 18 managed files and 9 restored profile files matched after
cleanup.
