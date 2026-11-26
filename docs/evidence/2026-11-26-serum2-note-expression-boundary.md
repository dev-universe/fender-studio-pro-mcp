# Serum 2 and Serum 2 FX VST3 Note Expression

R476 inspected the same Xfer-signed Serum 2 2.1.5 VST3 arm64 slice as the
[SDK interface inventory](2026-11-25-serum2-vst3-interface.md). The
isolated process for each class initialized and connected its own
component and editor controller. Both classes returned the SDK's
`INoteExpressionController` interface. The two method entry points for
count and descriptor information matched between classes.
[Steinberg interface reference](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1INoteExpressionController.html)

REA with Ghidra analyzed both entry points. Craft-Anything and LLVM
independently matched 56 and 108 bytes. The inspected entry points return
without reading descriptors when the event bus or channel is nonzero.
At bus 0 and channel 0, a component state check precedes access to its
current descriptor list. The manual describes the VST3 Note Expression
mode for compatible hosts as a separate mode from MIDI MPE.
[Xfer Serum 2 manual](https://www.xferrecords.com/manual/serum-2/docs)

Both isolated probes stopped at the first count request before a count
or descriptor was returned. The instrument process ran for 0.953
seconds and FX for 0.255 seconds. Studio Pro stayed closed and the
probes sent no note or parameter events. The actual descriptor list,
Studio Pro delivery, and per-note sound response remain unverified.
