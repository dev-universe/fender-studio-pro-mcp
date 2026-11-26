# Impact sample note lifecycle

R405 and R405B used Studio Pro 8.1.1 Build 113103 on macOS arm64,
one isolated Impact track and one MCP owner at a time. Main stayed muted
at zero and the transport stopped. An owned receive-only CoreMIDI source
sent channel-1 note-on/off pairs at velocity 42 for MIDI 37 and 36.
CoreMIDI loopback confirmed the four message bytes before each trial.

An original 14-second, 48 kHz, 440 Hz sample was loaded on bank A,
C1, MIDI 36. The editor showed frame bounds 0–672,000 and Normal
trigger mode. Its neighbor C♯1, MIDI 37, was empty. A Rumble Native
insert on Impact St 1 provided independently read input and output
peak parameters. Their native raw scale was 0–10,000.

| R405B observation | Raw downstream peak evidence |
| --- | --- |
| Idle and empty C♯1 held for 1.878 s | All four taps read exactly zero across 5 and 10 reads, respectively |
| Loaded C1 held for 1.880 s | All four taps read at least 220.7846 across 10 reads |
| After C1 note-off | Last two observations were 0.000027962 and 0.000015643 on all four taps by 8.153 s after release |
| Independent read-only recheck | All four peaks were exactly zero in two further reads |

The last two timed observations were more than six orders of magnitude
below the held level and occurred before the sample's 14-second end.
After monitoring was restored, a fresh owner bound the same document
and exact Impact and Rumble instances. Two full reads matched all
4,262 Impact DSP rows from before the MIDI input. The mixer and empty
timeline also matched.

R405 recorded the first positive tap and sent note-off on exit after
its verifier used the wrong peak range. The next trial corrected the
range, recorded the note-off decay, and stopped its timed loop at eight
seconds while the values were still above its absolute (10^{-6})
threshold. The independent recheck confirmed exact zero after that
window. The measured transition supports the Normal trigger behavior
without specifying an exact silence timestamp.

R405 completed 195 MCP requests in 223.052 seconds and had a
318.094-second app lifetime. R405B completed 234 requests in 176.329
seconds and the follow-up 78 in 10.908 seconds. Its app lifetime was
296.922 seconds. The two scratch songs closed without saving. Their
source bytes, the normal 18 managed files, and 11 restored profile
files per trial matched. Both temporary MIDI sources exited.

This verifies internal signal for one populated pad and one negative
neighbor in Normal trigger mode. Physical output, other trigger modes
and save/reopen have separate acceptance criteria.
