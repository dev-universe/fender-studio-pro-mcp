# Ampire global Speed alias on the default PAE Chorus 1

R357 used Studio Pro 8.1.1 build 113103 on macOS arm64. The exact
Ampire AudioEffect was on a muted empty scratch track with Main silent.
The default rack carried PAE Chorus 1 in slot 5.

The global Speed alias and `stomp5.vibratoRate` independently read
3000.0001192092896 raw at baseline. Writing only the global alias to
normalized 0.625 changed exactly both rows to raw 6250. Writing the
original finite bits once through the same alias restored both rows.
The other 88 non-meter rows, the original component fingerprint, mixer,
timeline, and insert chain matched. Wah, Volume and Pitch were zero
with empty display text on the default rack. Their target mappings
were left open.

A preceding read-only parameter-choices request for the slot selector
was rejected with an integer bounds error. The native row describes a
List with exact floating-point bounds 0.0 and 15.0. That discovery request
made no change. The fresh owner independently confirmed the original
fingerprint before the Speed changes.

The owners used 28 and 36 requests in 3.234 and 3.942 seconds. The
host ran 74.629 seconds and exited normally. The song stayed byte-identical.
All 18 managed files and 10 touched profile files were restored.
