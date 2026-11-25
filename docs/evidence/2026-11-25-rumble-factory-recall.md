# Rumble Native factory recall and empty processor slots

R321 exercised the built-in Rumble Native effect on Studio Pro 8.1.1 build
113103 on macOS arm64. Each test used its own empty scratch song and one
bound instance, with Main silent and transport stopped. The host catalog
issued 111 entries. The baseline in both tests was the factory preset
`001 BASIC BASSMAN CLN`. Each state was compared through two independent
reads of all 39 DSP rows and the serialized model and setting projection
for 26 processor slots.

| Target preset | Applied state | Result after recalling the baseline |
| --- | --- | --- |
| `010 BASIC TV FRONT` | The amp model and `output.level` changed. The editor showed the preset title. | The baseline title returned. All 39 DSP rows, 26 model slots, and 22 numeric setting encodings matched. Catalog and capture tokens were released. Both expired tokens were refused and the state still matched. |
| `100 A REMARK JACO MADE` | The amp and effect-slot models changed. The editor title and independent state agreed with the requested entry. | The baseline title returned. All DSP values and 22 numeric encodings matched. Two stable independent reads still reported the target model IDs in `effect2` and `effect3`. The baseline factory file explicitly declares both slots empty. |

The first case verifies exact restoration by factory recall for that pair.
The second establishes that an accepted recall and matching scalar reads
can leave serialized slot identities different. Its scratch stopped after
the two matching readbacks. Input gain did not change in either case. Prior
generic state-capsule restoration showed float drift, and this test used
factory recall for recovery. Audio and save/reopen persistence remain
unverified for these cases.

The two MCP owners used 90 requests in 42.842 seconds and 72 requests in
37.022 seconds. The respective host runs lasted 76.076 and 116.134 seconds.
Both owners and hosts exited normally. The scratch songs closed without
saving and their original bytes matched. The 18 managed MCP files and ten
Studio Pro settings and cache files were restored.
