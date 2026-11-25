# Scope state coverage receipt in Studio Pro 8.1.1

R382 exercised the public MCP server and candidate `87e932e` on macOS arm64,
Studio Pro 8.1.1 build 113103. Scope and Tone Generator were separately
bound on an owned stereo track. Tone Generator stayed off, the track had
no input or events, transport was stopped and both faders were at zero.

Scope capture and accepted restore both returned `scope_state_coverage`
with `capsule_includes_live_trace: false` and
`restore_may_clear_held_trace: true`. The restore still reported
`applied_unverified`. Separate parameter reads observed the threshold change
to 37.5% and its restoration. A fresh Scope capsule matched the original
fingerprint. Two Tone Generator captures matched each other and omitted
the Scope field, as did all release receipts.

The final 41 DSP rows matched their original values. All four tokens
were released. The owner sent 42 requests in 4.107 seconds. Studio Pro
closed without saving after 31.025 seconds, and the scratch bytes,
18 managed installation files and 10 application settings files matched
their originals. The separate trial had already verified what restore does
to an actual held waveform. [Held trace verification](https://github.com/dev-universe/fender-studio-pro-mcp/pull/171)
