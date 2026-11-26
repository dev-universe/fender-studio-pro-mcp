# Ampire activation aliases for five newly occupied slots

R356 exercised the exact Ampire insert on Studio Pro 8.1.1 build 113103,
macOS arm64. One owned, empty, muted stereo track was used with silent
Main and stopped transport. Document, session, and native instance were
checked before writes.

The default rack had no native `active` child in slots 2, 3, 4, 7 or 8.
The MCP inserted the same Compressor model in all five locations.
Every populated slot exposed its own writable Toggle alias and
particular `stompN.active` DSP child. Each test drove its own alias
`1 → 0 → 1`. Two independent reads after each write showed that only
the target alias and its exact native child changed. The editor
showed slot 2 off and on while displaying all five Compressors.

After each round trip the 114 stable fields and complete serialized
component fingerprint matched the prepared populated rack. Four
read-only meters and the identified read-only chorus LFO indicator
were outside stable-value comparison. The mixer, arrangement, and
insert chain matched throughout.

The test made 174 requests in 38.186 seconds. The host exited normally
after 69.554 seconds. The scratch was discarded and remained
byte-identical. All 18 managed files and 10 changed app-profile files
were restored and verified.
