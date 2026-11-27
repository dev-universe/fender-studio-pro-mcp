# Ampire recovered factory microphone basis after save and reopen

R430 used Studio Pro 8.1.1 Build 113103 on macOS arm64. The
owned scratch contained one exact MCM 800 / 4x12 M65 Ampire.
The transport was stopped and Main was muted at zero.

Six guarded writes reproduced the R427 recovery. The linked
microphone vector, 90 typed rows, 2,297-byte component, mixer
and track returned exactly. The owned song was saved once. An
independent archive read followed the exact insert reference to
its saved `.dsppreset`. The component bytes and microphone
records matched the original. All 90 live rows matched after Save.
[R427 recovery](https://github.com/dev-universe/fender-studio-pro-mcp/pull/216)

A new Studio Pro process and MCP session opened the saved file.
The prior document token was refused. The live linked state and
component again matched the original and the saved archive.
After a separate fresh check, one Link Off exposed the three
retained basis values in two independent reads.

| Mic | R351 initial factory Link Off | R429 after same-session recovery | R430 after Save and restart |
| --- | --- | --- | --- |
| A | `3F4BB3E9` | `3F4BB3E8` | `3F4BB3E9` |
| B | `3DD03F95` | `3DD03F94` | `3DD03F95` |
| C | `3DD2212A` | `3DD2212A` | `3DD2212A` |

Both original Float64 raw bits and normalized Float32 bits
matched for A, B and C. Only Link changed from the immediately
preceding linked state. The other 89 rows, mixer, track and
insert records matched. This establishes the recovered factory
basis after an ordinary Save and restart of this fixture.
Same-session Link Off has the separate result shown above.
[R429 same-session comparison](https://github.com/dev-universe/fender-studio-pro-mcp/pull/218)

The two owners made 166 and 63 requests in 77.929 and 27.492
seconds and released ten tokens. Both app processes exited
normally after 133.914 and 88.617 seconds. The Link Off was
discarded and the once-saved linked file was rehashed. The
original input, 18 managed files and ten restored app profile
files were independently checked.
