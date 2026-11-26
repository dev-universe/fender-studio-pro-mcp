# Scope sidechain assignment read in Studio Pro 8.1.1

Studio Pro 8.1.1 build 113103, macOS arm64. The host arm64 UUID was
`6B547378-B3BB-322A-87C9-901493FD2BED`. The isolated candidate was
`38c7a9a1cfef` with native UUID `D615728D-0C5E-389A-ACD3-9344197BE348`.
One MCP owner used the protected R573 scratch song with the transport stopped,
no input or armed audio tracks, and Main muted with zero volume.

The named `scope_sidechain_assignment_get` tool independently observed:

| Stock editor state | Named MCP tool, two reads | Independent mixer Sends |
| --- | --- | --- |
| No assignment | `source_absent` | Both racks empty |
| One Send assigned to the bound Scope | `assigned_to_exact_scope` | Source one to Scope, receiver empty |
| Send removed and Scope header restored | `source_absent` | Both racks empty |

The source and receiver labels were unique on the current mixer list. The
receiver was stock Scope insert slot zero, bound to its class and instance UUID.
A wrong source label, wrong receiver label and wrong instance UUID were refused.
A stale document and a missing document were refused before returning a result.
The native observation reads the saved destination through the build-pinned
PortParam getter, retains the port identity and checks ownership against the
exact Scope Device before publication.

All 31 Scope DSP rows matched the baseline after restoration. The fixture made
55 MCP requests in 98.674 seconds. The host exited with code zero after 128.385
seconds. The scratch closed without Save, its file digest stayed identical, and
18 managed files plus 10 app-profile files were reverified or restored. The two
Send changes were made in the stock editor. The other enum values describe an
unassigned Send and an assignment to a different destination. This trial
exercised the absent and exact-assignment states.

The published operation reports the authored mixer assignment. Reciprocal DSP
edge, signal delivery, broader Send racks and routing writes need their own
validation.
