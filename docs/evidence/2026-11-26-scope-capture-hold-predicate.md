# Scope capture and hold predicate

R540 examined the shipped Studio Pro 8.1.1 build 113103
`studioeffects` arm64 image. It pinpoints what Oneshot and
Retrigger change and what the next audio callback tests. The
prior R380B live test identified the two parameter tags and
observed silent arm, capture, hold and retrigger on screen.

| Step | Verified native behavior |
| --- | --- |
| Oneshot (`trigOneshot`, `0x5354724F`) | The setter stores the configuration bit in the DSP at `+0x1A40`. The getter at `0x3317C` exposes that saved bit. The setter also invokes the same rearm helper as Retrigger. |
| Retrigger (`trigRetrig`, `0x53547252`) | The setter reuses the configured mode and calls the rearm helper along the `0x32C98–0x32CAC` path. Scope's internal attachment slot `0x5067F8` also reaches this helper through `0x33C98`. |
| Rearm (`0x7D1C`) | Clears the latch at context `+0x555`, updates mode at `+0x52E` and, in Oneshot, clears the participating windows and updates a ring-relative basis. This helper does not write the count at `+0x558`. |
| Audio acquisition (`0x74F4`) | On a subsequent DSP invocation, when count is at least the configured window, it holds if the latch is set and either forced mode is set or configured mode is Oneshot. Otherwise that full-count branch clears count and latch before testing for new acquisition. |
| Start and progress | The normal audio trigger sets the latch and resets count at `0x7594–0x75A0`. The force path can do the same at `0x7650–0x765C`. The producer appends bounded samples, advances count at `0x7AA8` and uses a separate dirty byte for display work. |
| Reconfigure | The attachment operation at `0x5067E8 → 0x33B64` explicitly resets capture through `0xB2AC` before applying configuration. |

A full count can survive the Retrigger helper with the latch
cleared until processing resumes. The configured Oneshot bit
and the count alone therefore cannot prove that the current
capture is held or complete. The predicate applies at one
producer observation of the exact Scope instance. A reader still
needs a synchronized producer observation for count, latch,
valid window and generation. The statically identified fields
remain private.

REA/Ghidra inspected 18 procedure queries. Original Mach-O
disassembly and fixups matched 27 instruction words and three
rebases. Craft-Anything 0.6.1 independently matched ten
20-byte windows. R540 ran offline and REA was closed after
export. No host or plugin settings changed.
