# Pedalboard exchange of two Compressors

The R314 scratch song used Studio Pro 8.1.1 build 113103 on macOS arm64.
A diagnostic candidate used the same native slot editor as the public
adjacent exchange. It admitted a Compressor in each of slots 1 and 2.
The first had normalized Gain 0.324 and was active. The second had
normalized Gain 0.683 and was bypassed.

One exchange left both model labels unchanged. The editor drew the Gain
knobs and bypass lights in exchanged positions. The two
`controls.active[stompN]` values exchanged and the component fingerprint
changed. Separate DSP enumerations still returned the original
`stompN.gain` and `stompN.active` values at both positions. All unrelated
rows agreed. A fresh owner verified the same DSP result with the editor
closed and after reopening it.

The move trial used 67 requests in 41.436 seconds. The fresh verification
used 13 requests in 2.090 seconds. Studio Pro exited normally after
144.443 seconds. The scratch song closed without saving and the original
18 managed MCP files and ten Studio Pro settings and cache files were
restored. The public exchange still requires different models because
this diagnostic could not verify complete payload movement through its
independent DSP readback.
