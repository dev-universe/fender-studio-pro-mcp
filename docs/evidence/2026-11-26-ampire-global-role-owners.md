# Ampire global role ownership in Studio Pro 8.1.1

R561 used Studio Pro 8.1.1 build 113103 on macOS arm64 and candidate
`ccfaec0`. The scratch held one bound Ampire instance. Main and its
track were muted. The transport stayed stopped, with no MIDI or Save.

Two `plugin_parameter_details` reads with `source: dsp` and
`include_owner: true` agreed at each sampled state. All four global
rows were native `DeviceParam` objects with a `DeviceParameterList`
controller, and all three local rows used a `DeviceParamFolder`
controller. The public original-parameter chain ended at the same
self-reference for every row. The original chain therefore does not
resolve the plug-in's processing target, even for the two controls
with independently observed value propagation.

| Local change | Independently observed global change |
| --- | --- |
| FAT slot 2 Volume, 50% to 100% | No global row changed |
| Wah Wah slot 2 Pedal, 0% to 100% | Wah moved from 0% to 100% |
| PAE Chorus 1 slot 5 Vibrato Rate, 30% to 100% | Speed moved from 30% to 100% |

Global Volume and Pitch stayed at raw zero with an empty display in
all sampled states. The trial wrote only the model selectors, local
controls and the slot 2 activation residue. It did not identify a
Volume or Pitch role for these models or establish audible response.

Removing either test model left the otherwise vacant slot 2 active at
raw 10000. Restoring its exact activation row to zero recovered the
original 89 typed settings and original component bytes. The first
owner timed out before the MIDI reattachment prompt was dismissed.
The next two owners stopped at the activation residue and at the
already-populated Chorus slot. Each continuation independently bound
the same document and Ampire instance before proceeding. The final
Chorus test reused its existing model and restored the entire original
parameter and component snapshot.

The four owners issued 2, 99, 93 and 72 requests in 9.076, 8.910,
8.870 and 6.401 seconds respectively. Three capsules were explicitly
released. Owner teardown retired the remaining capsule, with no
capture files left. Studio Pro exited normally after 177.144 seconds.
The unsaved scratch hash matched. All 18 managed installation files
and ten changed profile files were restored.
