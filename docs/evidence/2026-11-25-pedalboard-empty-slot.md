# Pedalboard exchange with an empty slot

The R317 scratch song used Studio Pro 8.1.1 build 113103 on macOS arm64.
A diagnostic candidate admitted a move between Compressor in slot 1 and
empty slot 2. The editor showed Compressor in slot 2 and slot 1 empty
after one native request. The independent DSP reads confirmed the same
order, the full typed Compressor payload and all unrelated rows. The
component fingerprint changed.

The first reverse request included incorrect expected labels and the
native guard returned `not_applied`. A fresh MCP owner read the moved state
again, bound the same instance and issued one reverse request with the
correct labels. The editor and DSP model reads showed Compressor back in
slot 1 and slot 2 empty. One of the 33 DSP rows differed from the original
populated baseline. `controls.active[stomp2]` had raw float value 10000
and normalized value 1, compared with raw and normalized zero originally.
The visible rack returned to its original layout and exact parameter
restoration failed.

The forward trial used 63 requests in 45.760 seconds. The follow-up used 18
requests in 2.663 seconds. The host exited normally after 157.027 seconds
and the scratch song closed without saving. The original 18 managed MCP
files and ten Studio Pro settings and cache files were restored. The
public adjacent-exchange contract keeps its occupied-slot guard.
