# Ampire factory mic precision after an exact-bit correction

R362 ran Studio Pro 8.1.1 build 113103 on macOS arm64. One exact
MCM 800 / 4x12 M65 insert was used in a muted, empty, stopped scratch
with silent Main and stable document and instance bindings.

A linked write moved Mic B to raw 2500 and changed all three mix levels.
One accepted load of the captured original state reproduced the R350
result. The independent Mic A value was lower by
0.0005960464477539062 raw, Mic B/C matched, and the whole-component
fingerprint differed.

The trial then sent the original finite Mic A bits exactly once with
Link still active. Independent reads showed Mic A restored exactly.
Mic B and C each moved by -0.0002980232238769531 raw, the same two
values observed after re-enabling Link in R351. The complete hash
differed from both the original and the post-load capture.
The corresponding controls still displayed 80%, 10%, 10%. All other
stable settings, the mixer, timeline and insert chain matched.

This factory-state correction did not establish byte-exact component
recovery. The different prepared 25%, 25%, 50% baseline has separate
exact round-trip evidence. No other correction was dispatched.

The owner made 62 requests in 5.824 seconds and released three tokens.
The host exited normally after 40.569 seconds. The scratch was
discarded and remained identical. All 18 managed and 10 touched
app-profile files were restored and verified.
