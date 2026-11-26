# Ampire FAT recovery and the next Link-off after reopening

R495 used Studio Pro 8.1.1 build 113103 on macOS arm64 with MCP runtime
`ccfaec0`. One muted track held one Ampire insert and had no input,
sends, monitor, recording or events. Main was muted at zero.

The trial repeated the [FAT capsule recovery](2026-11-26-ampire-fat-linked-recovery.md).
The restored linked state matched the original 89 stable DSP rows and
the SHA-256 of the 2,297-byte component capsule. The owner verified the
same rows and capsule after a single Save on the owned scratch. The
separate [no-reopen comparison](2026-11-26-ampire-fat-link-off.md) had
previously found one Float32 unit of drift on microphones A and B
when Link was turned off in the original process.

A new Studio Pro process opened the saved scratch with the file
write-protected. The first owner used an older server executable and
the request schema refused the extra readback field before dispatch.
A fresh owner used the pinned executable. It verified a new bridge
session and document, the saved Ampire identity, all 89 linked rows
and the same capsule SHA-256. It then sent exactly one Link-off write.
Independent reads matched the factory first Link-off values on all
three microphones. No other stable row changed.

| Microphone | Factory and reopened Link-off Float32 bits | ULP change |
| --- | --- | ---: |
| A | `3F4BB3E9` | 0 |
| B | `3DD03F95` | 0 |
| C | `3DD2212A` | 0 |

The observation covers this FAT fixture and one Save and cold reopen.
The first host used 135 requests in 32.713 seconds. The two reopen
owners used 20 requests in 2.411 seconds and 28 requests in 3.422
seconds. The hosts exited normally after 67.856 and 78.366 seconds.
No further Save was issued. The original song, saved scratch, 18
managed files and ten app profile files were independently checked.
