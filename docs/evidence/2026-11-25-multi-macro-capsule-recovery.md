# Multi Instrument Filter Env capsule recovery

R393 reused the exact saved two-Mai-Tai fixture in Studio Pro 8.1.1
build 113103 on macOS arm64. Two initial state captures had the same
fingerprint. Writing the bound Filter Env macro to 5000 changed only
the declared amount parameter on each child. A separate capture
detected the changed state.

The existing `plugin_state` restore accepted the original token
and returned `applied_unverified`. The parent instance and the two
logical child slot IDs stayed the same. Both live child Device IDs
changed. After rebinding the Devices, two independent reads matched
all 566 original child rows, including the two original raw amounts
of 5003.644824028015.

The parent Filter Env macro was the only changed row among 150
root rows. Its exact initial value 5449.993014335632 became
5003.644824028015. A new capture did not match the initial snapshot.
The capsule therefore recovers the tested child state and leaves
the parent pilot value different. The [root-only restore](2026-11-25-multi-macro-restore-boundary.md)
had the opposite limitation in R392.

All four tokens were released. The owner took 80 requests in 7.166
seconds, and the app exited normally after 44.483 seconds. The
scratch closed without saving and was byte-identical. All 18 managed
files and nine profile files were restored.
