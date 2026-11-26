# Serum 2 preset inspection

R479 ran the Rust MCP server over stdio on macOS arm64 with Studio Pro
closed. Four retained test files covered the instrument and FX in
editor-native `.SerumPreset` and processor-only `.vstpreset` form.
Each reply matched the independently read variant, file length, SHA-256,
and applicable load method. File hashes were unchanged after the calls.

Both editor files used Zstandard frames with a window descriptor and
without a frame content size. The inspector checks the native format's
outer size declaration and window against the existing 16 MiB limit.
The processor-only VST3 path requires an embedded frame content size
matching the declared size. This difference was confirmed by the test
files and the [Zstandard frame specification](https://datatracker.ietf.org/doc/html/rfc8878#section-3.1.1.1.1.1).

The server also rejected a renamed editor file, a symlink, a file larger
than the limit, and a relative path. All eight calls and the tool listing
completed in 1.564 seconds, with normal server exit. The checks cover the
file envelope and state header. Decoding or validating the preset body,
and observing its applied settings in the host, are separate checks.
