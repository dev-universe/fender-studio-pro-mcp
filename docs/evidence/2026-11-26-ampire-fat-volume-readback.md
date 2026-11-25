# Ampire FAT local Volume readback

R469 ran in Studio Pro 8.1.1 build 113103 on macOS arm64. In a muted,
empty scratch, Ampire's second stomp slot started vacant. The bounded
run independently confirmed FAT at the previously verified index 5,
then read `stomp2.volume` tag 12293 at 50%, 75%, and 50% in two
independent passes per setting. The resulting raw values were 5,000,
7,500, and 5,000. All other FAT-state rows matched during the Volume
change. Its component fingerprint matched after Volume returned.

Ampire's global `controls.volume` and `controls.pitch` remained raw
zero with empty display strings throughout. No global alias writes or
audio tests were performed. This is evidence for the exact local FAT
control and its separate readback from the two global aliases.

Returning the model selector to its original vacancy removed the FAT
parameter rows, but the writable `controls.active[stomp2]` row stayed at
10,000 compared with initial zero. The read-only tuner row also changed.
The run stopped without claiming full original component restoration.

The prior diagnostic build's list query refused floating-point list
bounds. The run used the established FAT index from the same Studio Pro
build and verified the active model and its local row independently.
The shipping list reader separately supports integral floating-point
bounds, as validated by the earlier [native list choices PR](https://github.com/dev-universe/fender-studio-pro-mcp/pull/149).

The three guarded owners made 24, 16, and 55 requests. Total owner time
was 11.191 seconds. Studio Pro exited normally after 98.417 seconds.
No playback, MIDI, or Save was dispatched, and 18 managed files and 10
app profile files were restored and verified.

R470 rebuilt the current `main` at `ccfaec0` and repeated the installed
8.1.1 test. The list query confirmed all 16 choices and `FAT` at index
5. The same local Volume sequence and unchanged global alias readings
were reproduced. The FAT component remained 2,371 bytes with an
identical hash before and after the local Volume change.

After returning the slot to vacant, the original same-instance component
load was acknowledged. Independent reads confirmed the vacant activation
value returned. The writable `cab.micmixA` normalized value shifted by
one Float32 ULP, from `3F4BB3E9` to `3F4BB3E8`. The resulting component
had 2,294 bytes compared with the original 2,297 bytes. The trial kept
this as an incomplete full-component restoration. It made 83 requests
in 7.874 seconds. Studio Pro exited normally without saving. All 18
managed files and 10 app profile files were restored and verified.
