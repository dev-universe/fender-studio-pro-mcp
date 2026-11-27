# Scope Oneshot waits for a qualifying signal in Studio Pro 8.1.1

R380B used Studio Pro 8.1.1 build 113103 on macOS arm64. Tone Generator
fed Scope on an owned stereo track with no input or events. Both plugin
instances were individually bound. The track and Main faders were zero,
transport was stopped and arming and monitoring were off.

The native formatter and separate current reads verified the oscillator
frequency, level and Scope threshold. A fixed Scope viewport independently
showed the following sequence on channel A.

| Stage | Bound oscillator and Scope values | Visible trace |
| --- | --- | --- |
| Baseline | Tone off, Oneshot off | Flat |
| Waiting | Tone off, Oneshot on, threshold 37.5% | Flat |
| Subthreshold input | 440 Hz, −24 dB, Oneshot on | Flat |
| First qualifying input | 880 Hz, −7.50 dB, Oneshot on | Stable waveform with approximately 75 px between crests |
| Source changed | 440 Hz, −7.50 dB, Oneshot on | The same held waveform and crest locations |
| One Retrigger | 440 Hz, −7.50 dB, Oneshot on | New waveform with approximately 134 px between crests |
| Restored | Tone off, Oneshot off, threshold 0.0% | Flat |

The approximate pixel spacings distinguish the two canvas traces in the
same viewport. The frequency labels were read from Tone Generator.
The `retrigger_scope` action receipt had `effects_verified=false`.
The view on the same editor independently showed the completed trace
replacement. The observations establish the functional waiting, capture,
hold and restart sequence. The exact internal trigger flag transition
timestamp and hidden capture-buffer bytes remain unobserved.

Two final complete reads matched all 41 original DSP rows across the two
inserts. The mixer and track matched, the scratch was closed without saving
and stayed byte identical, and 18 managed installation files and 10 app
settings files were restored. The owner completed 90 requests in 91.337
seconds. The app exited normally after 117.776 seconds.
