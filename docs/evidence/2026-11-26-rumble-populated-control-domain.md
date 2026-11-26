# Rumble Native controller queries after populated preset recalls

R494 used Studio Pro 8.1.1 build 113103 on macOS arm64 with MCP source
`59ac9e0`. One Rumble Native insert was bound by exact class and current
instance on an owned scratch track. Main was muted at zero, the track
was silent, and transport, external input, monitoring and recording
stayed off.

MCP recalled `001 BASIC BASSMAN CLN`, `100 A REMARK JACO MADE`, and
`001 BASIC BASSMAN CLN` using a single issued factory catalog. Each title
and the complete model and settings projection were observed
independently before the controller queries.

| Stage | Indexed controllers | Names looked up | Names resolved | Current category/effect rows |
| --- | --- | ---: | ---: | --- |
| Initial 001 | 26 present, zero indexed parameters | 22 | 0 | 0 / 0 |
| 100 | 26 present, zero indexed parameters | 39 | 0 | 0 / 0 |
| Recalled 001 | 26 present, zero indexed parameters | 22 | 0 | 0 / 0 |

The current catalog was queried with the editor closed. The indexed
and named controller reads agreed with the exact serialized model
manifest. All four query results matched separate calls at each stage
and conferred no mutation capability. The 35 stable DSP rows and
numeric settings remained the same around the reads. The final state
retained the two previously observed model IDs in `effect2` and
`effect3`. [Prior factory recall](2026-11-25-rumble-factory-recall.md)

The observations cover the current named, indexed, and selector paths
in this installation. They establish an absence in those paths with
two populated presets. A positive model list and a writable
model-specific controller remain unverified.

The owner made 101 requests in 8.252 seconds. Studio Pro exited
normally after 59.571 seconds. The catalog token was released and
the scratch closed without saving. The original scratch, all 18
managed installation files, and ten app profile files were checked.
