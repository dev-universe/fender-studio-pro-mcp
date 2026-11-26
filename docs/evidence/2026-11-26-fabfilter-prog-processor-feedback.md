# FabFilter Pro-G processor feedback and controller readback

R473 used the signed FabFilter Pro-G 1.42 VST3 on macOS arm64. The
verified slice matched the earlier R259 Pro-G analysis byte for byte.
Two isolated SDK hosts each connected the component and edit controller
for that same product class. The hosts opened no audio device and used
no Studio Pro process or MCP writes.

Each processor consumed a 0.035 test point for its legacy control. Both
emitted one point with the same parameter ID and value
`-0.44444441795349121`. The controller still read 0.035 before delivery.
In the control probe, which skipped forwarding the point, the next
controller read was still 0.035. In the second probe, forwarding the
processor point through Float32 and back to Float64 returned
`kResultOk` from the controller. An independent controller read
returned zero. Both probes explicitly restored the baseline. The test
control and one companion read zero, and the component state matched
its initial bytes.

Both comparison processes exited normally. Their measured times were
0.228 and 0.222 seconds. This demonstrates one way that an isolated
Pro-G controller returns to zero. The earlier
[R304 stopped transport observation](2026-11-24-fabfilter-legacy-readback.json)
also settled at zero. Whether Studio Pro sent the same point to the
controller in R304 still needs an actual host observation. This adds
evidence for controller behavior and no new MCP operation.
