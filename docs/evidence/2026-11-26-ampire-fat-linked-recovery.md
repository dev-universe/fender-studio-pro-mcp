# Ampire FAT component recovery in Studio Pro 8.1.1

R477 used Studio Pro 8.1.1 build 113103 on macOS arm64. One isolated
scratch had a single Ampire instance and a vacant second stomp slot.
The runtime was built from `ccfaec0`. Its source matched the code at
`ad31cb7`. With the transport stopped and Main muted at zero, the trial
read all 16 model choices and repeated FAT Volume at 50%, 75%, and 50%.
The two FAT captures matched at 2,371 bytes. The global Volume and
Pitch aliases stayed at raw zero.

Returning the model to vacant left the slot activation set. Restoring
the initial capsule returned that activation to zero. The next capture
had 2,294 bytes and linked microphone A was one Float32 unit below its
original value. The independent reads matched the earlier
[FAT observation](https://github.com/dev-universe/fender-studio-pro-mcp/pull/261).

The trial switched Link off and independently read the retained gains.
A was already at the verified preparation value, so the trial wrote only
B while unlinked. It turned Link on and wrote the exact original C.
Each of the four writes had a matched readback and two independent
complete observations of the same instance. The other settings matched
through all four writes. After the final C write, all 89 stable settings
matched the initial read, and the 2,297-byte capsule matched its original
SHA-256. The later behavior of the retained gains when Link is toggled
again remains unverified.

The first owner stopped after two requests at the startup MIDI device
notice. The operator declined reconnecting the absent devices and checked
the exact scratch window. The next owner made 125 requests in 12.182
seconds. Studio Pro exited normally after 72.317 seconds without Save,
playback, or MIDI dispatch. Both owners exited. All 18 managed files
and 10 app profile files were restored and rechecked.
