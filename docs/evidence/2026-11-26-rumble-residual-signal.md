# Rumble Native signal after factory recall

R491 used Studio Pro 8.1.1 build 113103 on macOS arm64 and MCP source
`59ac9e0`. One stereo track held Tone Generator, an upstream Rumble
meter, the target Rumble, and a downstream Rumble meter in that order.
The transport was stopped. Main was muted at zero, the track was at
zero, and external input, monitoring and recording were off.

MCP recalled `001 BASIC BASSMAN CLN`, `100 A REMARK JACO MADE`, and
`001 BASIC BASSMAN CLN` on the same exact middle Rumble, using the
issued 111-entry catalog. An independent read confirmed each current
title. At each stage the source supplied the same 440 Hz sine at
−24 dB. Each phase had one Tone On and one Tone Off, six paired stereo
input/output reads, and independent quiet return. Left and right
agreed to the reported precision.

| Preset | Input median | Output median | Output/input |
| --- | ---: | ---: | ---: |
| Initial 001 | 668.1991 | 749.2025 | 1.12123 |
| 100 | 668.2111 | 1547.9375 | 2.31654 |
| Recalled 001 | 667.7635 | 749.6191 | 1.12258 |

Two independent state reads confirmed all 26 processor slots at each
stage. After the final recall all 35 stable DSP rows and 22 numeric
settings matched the initial state. Four read-only peak rows were
allowed to decay below the quiet limit. As in the earlier
[factory recall](2026-11-25-rumble-factory-recall.md), only
`effect2` and `effect3` retained different serialized model IDs.
The restored 440 Hz output/input ratio was 100.12% of its initial
value. This measurement covers peak amplitude at one frequency and
level. Phase, spectrum, and save/reopen behavior remain unverified.

The two observer instances kept their processor and settings
projections. Using the first observer's ID for the middle slot still
returned `not_applied`. This trial used the
[numbered-instance fix](https://github.com/dev-universe/fender-studio-pro-mcp/pull/272).
The owner used 468 requests in 50.605 seconds. Studio Pro exited after
80.567 seconds. The scratch closed without saving and stayed
byte-identical. All 18 managed files and ten changed app profile
files were restored and independently checked.
