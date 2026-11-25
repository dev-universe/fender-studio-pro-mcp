# Ampire DSP bypass and host activation indicators

R361 used one exact Ampire AudioEffect in a muted empty stereo scratch
on Studio Pro 8.1.1 build 113103, macOS arm64. Main was silent and
transport, monitor and record arm were off. The target document,
bridge session, host slot and native instance were bound.

Setting the native DSP `bypass` Toggle (tag 1) produced raw 1 on two
separate reads. It was the only changed one of 90 non-meter rows.
The Ampire editor displayed the orange bypass control and retained
the blue host activation indicator. The host slot reported `bypassed: false`
and the rack reported `bypassAll: false`. The entire
serialized component fingerprint also matched the original while the
DSP bypass was set.

Writing raw 0 through the same exact DSP path returned the screen and
all 90 rows to the original state with the same component fingerprint.
Mixer and timeline were stable. The unchanged fingerprint in the active
bypass phase limits what a component capture can establish about this
live setting. Save/reopen behavior and physical audio were outside the
test.

The owner used 63 requests in 35.035 seconds and released three tokens.
The host exited normally after 58.684 seconds. No song was saved. The
original scratch bytes and the 18 managed and 10 touched app-profile
files were independently checked after restoration.
