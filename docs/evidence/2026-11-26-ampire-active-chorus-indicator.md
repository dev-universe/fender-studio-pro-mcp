# Ampire active Chorus Speed and phase indicator

R452A used Studio Pro 8.1.1 build 113103 on macOS arm64 and the same Ampire
class and candidate as the [global Speed trial](2026-11-25-ampire-speed-alias.md).
The scratch had one muted audio channel with no hardware input or sends.
Main was at zero. The transport was stopped and the timeline was empty.

The MCP enabled the existing slot 5 PAE Chorus 1 once through its rack
activation alias and observed the exact child activation. While the Chorus
was active, one public Speed write changed the global alias and
`stomp5.vibratoRate` from raw `3000.0001192092896` to `6250`. A single raw-bit
write restored both rows. One deactivation restored both activation rows to
zero. The six measured windows covered active baseline, active changed and
active restored Speed, two windows per condition.

Each window contained 12 distinct measurements of the read-only
`stomp5.lfoIndicator` from the complete, identity-bound 94-row DSP page. The
windows spanned 2.42–2.46 seconds. The measurement records a varying phase
indicator at all three settings. Audio-rate modulation, period in hertz and
audio output were outside the measurement.

After deactivation, one read reported the original 89 fixed non-meter rows
and a residual phase indicator of raw `1129.0789395570755`. This failed the
original test's assumption that an inactive indicator would return to zero.
A fresh read-only owner pinned the same session, document and Ampire instance.
It observed that exact indicator value six times, verified all 89 fixed rows
and captured and released the original byte-identical 2,297-byte component
fingerprint. The same editor showed PAE Chorus 1 off with its modified marker
still visible. The retained indicator and editor marker are separate from
the recovered parameter and component state.

The main owner made 210 requests in 41.080 seconds. The read-only follow-up
made 24 requests in 4.138 seconds and issued zero setters or playback actions.
The scratch was closed without saving and Studio Pro exited normally. The
original song, all 18 managed installation files and ten profile files were
verified after restoration.
