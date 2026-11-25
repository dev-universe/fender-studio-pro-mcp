# Pedalboard empty-slot active alias after state reload

R341 used Studio Pro 8.1.1 build 113103 on macOS arm64 with the exact-build
diagnostic slot exchange. An active Compressor with normalized Gain 0.324
started in slot 1, with slot 2 empty and Phase Meter after Pedalboard.
The owned scratch had no events, the transport was stopped and Main was
silent.

The diagnostic moved the Compressor from slot 1 to slot 2 and back using
two individually guarded exchanges. The editor, model labels and complete
Compressor payload confirmed the outward movement. After the inverse, the
editor and captured component fingerprint returned to their original
state. The sole difference in the 33-row DSP read was the empty slot's
`controls.active[stomp2]`, which read 10000 instead of the original 0.

One exact-instance reload of the capsule captured after the inverse
returned the stale alias to 0. Two independent reads matched all 33
original rows. The component fingerprint stayed identical before and
after that reload. The editor, Phase Meter, mixer and timeline remained
unchanged during the reload. Five capsule tokens were released.

This verifies a same-session recovery for the tested vacant slot. It
uses a native state load. There was no automatic reload and the public
slot exchange still requires adjacent occupied slots with different
models.

The run used 95 requests over 39.491 seconds. The app ran for 75.496
seconds. The scratch file was byte-identical after closing without a
save. Ten changed profile files and all 18 managed files were restored.
