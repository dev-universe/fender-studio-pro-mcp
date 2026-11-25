# Public Pedalboard same-model exchange after Save and reopen

R444 exercised the public `pedalboard_slot_swap` on Studio Pro 8.1.1 Build 113103 for macOS arm64. It used the clean R343 candidate `d894e629f57c37d8ce3d070128cd783c69a05cb0` and a fresh owned copy. Both target slots contained Compressor. The request bound the exact document, instance, displayed models and fresh rack digest.

| Observation | Slot 1 normalized Gain / active | Slot 2 normalized Gain / active |
| --- | --- | --- |
| Before the exchange | 0.3240000009536743 / on | 0.6830000281333923 / off |
| Slot-specific DSP after exchange and Save in the first process | 0.3240000009536743 / on | 0.6830000281333923 / off |
| Editor and serialized payload after exchange | 0.6830000281333923 / off | 0.3240000009536743 / on |
| Independent DSP after a fresh process opened the file | 0.6830000281333923 / off | 0.3240000009536743 / on |

The editor comparison used the knob positions and activation indicators. The exact numeric values came from the DSP and serialized reads. The two active aliases already reflected the change in the first process, while the four direct slot rows retained their original values through Save. Independent file inspection matched the exchanged payloads and the other six positions. The full component digest matched in the fresh process and all 38 typed rows matched the expected reopened state. The Phase Meter, mixer and empty timeline matched. One public request carrying the prior document was refused before dispatch.

The two owners processed 94 requests in 55.672 seconds and 45 requests in 17.553 seconds. The hosts ran for 88.056 and 61.037 seconds and exited normally after declining the fresh process’s Save prompt. The protected original, once-saved file, 18 runtime files and ten profile files were verified at closure.
