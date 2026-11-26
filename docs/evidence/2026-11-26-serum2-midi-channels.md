# Serum 2 VST3 reference across sixteen MIDI channels

R502 used the Xfer-signed Serum 2 and Serum 2 FX 2.1.5 arm64 VST3 in
two isolated SDK hosts. Each connected controller reported one input
event bus with 16 channels. Each returned 2,050 assignments, with every
returned ID present once in that variant's parameter catalog. Both
processes exited normally, each in under one second.

CC 0, CC 2–127 and channel pressure 128 use the same assignment on
both variants. Add 500,000,000 to the controller number and one million
times the zero-based channel. On channel 0, CC 1 resolves to 516,000,000
and pitch bend 129 resolves to 516,000,001. The two special mappings are
qualified on channel 0. The REA/Ghidra procedure and independent
Craft-Anything and LLVM comparison underpin this arithmetic path.
[First-channel and static verification](2026-11-26-serum2-midi-reference.md)

Six saved readback sweeps from R499 covered 2,623 DSP rows each in
Studio Pro 8.1.1 build 113103. Every supported ID appeared as
`vstp<ID>` with tag `ID + 1` in each corresponding sweep of the two
signed native instances. Studio Pro exposed 2,082 raw MIDI-related
parameter IDs per variant. Those rows include the two global IDs and
the complete arithmetic ranges for channels 0–15. Physical MIDI and
higher-channel event delivery are outside this lookup verification.

A fresh MCP process independently matched all 4,100 supported results
and rejected 64 out-of-scope requests, including the two special
controllers on channels 1–15. Both explicit and omitted channel 0
returned the existing first-channel result. The verification completed
in 47.450 seconds, with Studio Pro closed during the stdio run.
