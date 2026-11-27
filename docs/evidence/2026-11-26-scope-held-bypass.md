# Scope held display across insert bypass

R562 used Studio Pro 8.1.1 build 113103 on macOS arm64 with candidate
`ccfaec0`. The owned scratch held Tone Generator followed by Scope on
one silent track. Playback stayed stopped. Main was muted and both
channel levels stayed at zero. The exact Scope slot was bound to its
current document and native instance for both bypass requests.

Scope was flat with the oscillator off and Oneshot armed. One 41.3 Hz
oscillator stimulus at −7.50 dB exceeded the 37.5% trigger threshold.
After confirming Tone Off by independent read, the editor still showed
a non-flat held trace at Hold 200%.

Turning on `plugin_bypass` for slot 1 set the host insert's bypass
flag. Its editor dimmed and the trace became flat. The 31 Scope DSP
rows remained present. Each row became disabled and lost its writable
and edit-support flags. Their raw values, normalized bits, labels and
order stayed equal, including raw zero on the row named `bypass`.
The MIDI-free upstream Tone settings and mixer were unchanged.

A single inverse bypass request restored the original host route and
all 31 enabled DSP rows. The Scope editor was active again and the
trace remained flat while Tone was still Off. Two consecutive returned
window captures were byte-equal. The earlier held screenshot differed.
For this fixture, host insert bypass does not preserve the held trace
across return. The renderer does not establish the producer callback
boundary or the lifetime of raw samples.

The run restored 10 Tone and 31 Scope rows and confirmed the original
host route, mixer and timeline. The owner issued 109 requests in
59.690 seconds. Studio Pro exited normally after 120.693 seconds,
without Save. The scratch archive was unchanged, and all 18 managed
installation files and ten changed profile files were restored.
