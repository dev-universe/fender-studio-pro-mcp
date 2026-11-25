# Ampire activation aliases for the default occupied slots

R355 exercised one exact MCM 800 and 4x12 M65 Ampire insert on Studio
Pro 8.1.1 build 113103, macOS arm64. The stereo scratch track was
empty and muted, the transport stopped, and Main silent.

| Slot | Model | Activation roles |
| --- | --- | --- |
| 1 | Tube Dreamer | `controls.active[stomp1]` and `stomp1.active` |
| 5 | PAE Chorus 1 | `controls.active[stomp5]` and `stomp5.active` |
| 6 | Delay | `controls.active[stomp6]` and `stomp6.active` |

Each MCP alias was written to 1 and then 0. Independent typed reads
reported both the alias and its named child returning the same values.
The editor showed the Tube Dreamer indicator in both positions and
the PAE Chorus 1 indicator in the active position.

The first owner stopped at the active Chorus because a read-only LFO
indicator changed between the two full reads. A fresh owner observed
the same exact instance and state, excluded only that identified
indicator and the four read-only meters from stable comparison, and
verified the paired values and exact return. The full capsule
fingerprint and all 89 stable rows returned. Mixer and arrangement were
unchanged. Other slots, audio processing and persistence were outside
this test.

The two owners made 57 and 65 requests in 33.109 and 6.453 seconds.
The app exited normally after 108.613 seconds. The scratch was
discarded without saving and remained byte-identical. All 18 managed
files and 10 touched profile files were restored.
