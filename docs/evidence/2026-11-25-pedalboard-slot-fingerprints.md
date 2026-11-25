# Pedalboard slot fingerprints from a component capture

`plugin_state` with `action: "capture"` adds `pedalboard_rack` when the
exact Pedalboard AudioEffect provides a recognized component state. Its
eight one-based entries each give the serialized model class and a
SHA-256 fingerprint of the slot's canonical JSON payload. Vacant slots
carry nulls for both fields. The fingerprint follows the slot payload
when it moves. Compare the field across captures from the same instance
and release each capture token after use.

R342 used the shipping adjacent exchange path on Studio Pro 8.1.1
build 113103, macOS arm64. The empty rack had eight vacant entries.
Two Compressors with distinct Gain and active values shared a class
and had distinct payload fingerprints. The ordinary capture from the
Phase Meter peer omitted the field.

The Compressor and Reverb were then exchanged using the public MCP
command. A fresh capture placed both identical payload fingerprints at
their new positions while the other six slots remained vacant. The
editor showed the same order. An inverse exchange restored all eight
entries, the entire component fingerprint, the typed DSP rows, and the
editor. There were no state-load requests. This observes serialized
settings independently of the controller values, which were stale
temporarily in the separate identical-model and vacant-slot trials.

The app ran for 68.062 seconds. The owner used 95 requests over 29.691
seconds. All six tokens, including the Phase Meter capture, were released.
The scratch was closed without saving and retained its original bytes.
Ten changed profile files and 18 normal managed files were restored.
