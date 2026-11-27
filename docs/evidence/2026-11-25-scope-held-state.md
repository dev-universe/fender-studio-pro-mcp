# Scope component restore clears the held trace in Studio Pro 8.1.1

R381 used Studio Pro 8.1.1 build 113103 on macOS arm64, with a Tone
Generator feeding the exact Scope instance on an owned stereo track.
The track had no audio input or events, transport was stopped, arming and
monitoring were off and both track and Main faders were zero.

Scope captured a 440 Hz oscillator setting once at −7.50 dB and a 37.5%
Oneshot threshold. Retrigger then captured an oscillator set to 880 Hz.
The editor independently showed different held traces. Scope returned the
same 31 DSP rows at both points. Two `plugin_state` captures at each point
returned the same 762-byte `component_alias_IStorable` capsule fingerprint
`227b52564fe6052c01cfda9fdec246b6014743bee42e5dea89ba867e26770375`.

Turning off Tone Generator left the second trace visible. Restoring the
first capsule then made the trace flat while Oneshot and the 37.5%
threshold stayed set. The first trace did not reappear. A fresh capture
returned the same fingerprint and all 31 Scope settings still matched.
The component capsule therefore does not distinguish the two observed
waveforms. An accepted component restore resets the visible hold in this
configuration.

Five capture tokens were released. The final canvas and all 41 original
DSP rows across the two inserts were restored. The mixer, track and scratch
bytes matched, and the 18 managed installation files and 10 application
settings files were restored. The owner completed 99 requests in 64.265
seconds. The app closed without saving after 86.820 seconds.
