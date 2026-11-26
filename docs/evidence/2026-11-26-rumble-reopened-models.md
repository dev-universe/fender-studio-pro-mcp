# Rumble Native retained processor models after cold reopen

R493 used Studio Pro 8.1.1 build 113103 on macOS arm64 with MCP source
`59ac9e0`. The owned song had Tone Generator and three Rumble Native
inserts. Transport stayed stopped and the source stayed Off. Main was
muted at zero, with no external input, monitoring or recording.

The same middle Rumble accepted `001 BASIC BASSMAN CLN`, then
`100 A REMARK JACO MADE`, then `001 BASIC BASSMAN CLN`. Each current title
was read independently. The last recall restored the 35 stable DSP rows
and 22 numeric settings. As in the earlier
[recall trial](2026-11-25-rumble-factory-recall.md), two model IDs
remained in the factory-empty `effect2` and `effect3` slots. The owner
released its catalog token. A single Save changed the owned song file
and produced a valid archive. The open song's repeated Rumble observation
matched the pre-Save state.

The saved file was reopened in a new Studio Pro process, with an
independent write restriction on the saved file. The bridge session and
document token were new and the insert instance IDs were the same.
The first owner stopped when a same-class peer's numeric settings
digest differed. A fresh read-only owner separately compared the
target and both peers.

- All 26 model IDs in the reopened target matched the pre-Save IDs,
  including the retained IDs in `effect2` and `effect3`. Both peers'
  model lists matched their own pre-Save lists.
- The target's 35 stable DSP rows still matched. In each of the three
  instances, the same five of 22 serialized numeric settings changed
  by one or two Float64 units. The table shows the signed changes
  in ULPs. The remaining 17 settings per instance matched.

| Setting | ULP change across reopen |
| --- | ---: |
| `amp.blend` | +1 |
| `amp.mvol` | −1 |
| `amp.treb` | +2 |
| `amp.volume` | +1 |
| `effect1.level` | −1 |

The result covers the processor state observed after the new host loaded
the song. The cause of bit-level drift and the reopened audio response
remain unverified.

The first owner used 123 requests in 29.449 seconds. The first reopen
owner used 11 requests in 1.827 seconds and the independent reopen owner
used 22 requests in 2.561 seconds. The two hosts exited after 78.618 and
67.542 seconds. There was one Save on the owned scratch, with the
original source and the saved copy verified at closure. All 18 managed
files and ten app profile files were restored.
