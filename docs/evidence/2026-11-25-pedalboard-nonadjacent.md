# Pedalboard nonadjacent exchange and sequential move

In Studio Pro 8.1.1 build 113103 on macOS arm64, the R315 scratch rack held
Compressor, Reverb and Big Fuzz in slots 1, 2 and 3. The diagnostic allowed
a direct request between slots 1 and 3. The stored Compressor gain was
changed before the trial to distinguish its payload.

| Operation | Slot 1 | Slot 2 | Slot 3 |
| --- | --- | --- | --- |
| Baseline | Compressor | Reverb | Big Fuzz |
| Direct exchange of 1 and 3 | Big Fuzz | Reverb | Compressor |
| Sequential exchange of 1 and 2, then 2 and 3 | Reverb | Big Fuzz | Compressor |

The editor confirmed both orders. The direct exchange preserved all the
middle pedal's values. Its seven rows acquired different enumeration
indices because the number of parameters in the first slot changed.
Each exchange preserved the complete moved pedal settings. The original
42 non-meter typed rows, including their original indices, and the
component fingerprint returned after reversing each sequence. Two read-only
output peak values varied and were excluded from the setting comparison.
The Phase Meter peer, mixer, inserts and empty timeline were preserved.

The final owner used 80 requests in 33.990 seconds. The app exited normally
after 270.298 seconds, including fixture preparation and the initial
comparison that treated changed row indices as changed settings. The scratch
song closed without saving. The original 18 managed MCP files and ten
Studio Pro settings and cache files were restored. These observations cover
the tested three positions without crossing a divider.
