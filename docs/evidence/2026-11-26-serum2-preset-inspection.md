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

R480 exercised the inspected SHA-256 as the application precondition
in Studio Pro 8.1.1 build 113103 on macOS arm64. The candidate MCP
server read each processor-only carrier, then applied it once to the
matching instance of the signed Serum 2 2.1.5 VST3. Each host receipt
matched the expected instance and digest. Both source files stayed
unchanged.

The first comparison stopped after counting changed FX access flags
as value changes. A fresh owner rebound the same session, document
and both native instances without reapplying. Its independent
2,623-row reads matched the post-apply snapshots. The instrument had
15 changed values. FX had 13 changed values and 16 additional rows
whose values matched while their access changed from writable to
read-only. Each peer matched its prior state after the other apply.

The two owners made 229 requests in 25.807 seconds. Studio Pro ran
for 106.606 seconds and closed without Save, MIDI or playback. The
temporary signed VST3 was retired, and 18 managed files and 10
profile files were restored and verified.
