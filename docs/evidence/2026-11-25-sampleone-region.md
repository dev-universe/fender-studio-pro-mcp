# SampleOne per-sample frame edit and recovery

R364 used Studio Pro 8.1.1 build 113103 on macOS arm64. One exact
SampleOne AudioSynth instance held a generated 12,000-frame WAV in
an owned scratch song with silent Main, stopped transport and no events.

The Wave editor initially showed Start 0 and End 12000. Entering 256
in the selected sample's Start field moved the boundary and preserved
the visible end. Two independent MCP reads showed all 235 global DSP
rows still matched. The serialized component changed from 5599 to
5601 bytes and had a different hash. This demonstrates why the scalar
DSP inventory alone cannot witness this per-sample edit.

One load of the captured pre-edit program returned the same visible
sample and waveform with Start 0 and End 12000. A fresh capture matched
the original loaded-program hash. The 235 rows, mixer, timeline,
document and instance were also checked. Four tokens were released.

The owner made 70 requests in 98.884 seconds and exited normally.
The app ran 128.707 seconds. The scratch was discarded without saving
and remained byte-identical. All 18 managed files and 10 touched
app-profile files were restored. Save/reopen and audible playback
were outside this trial.
