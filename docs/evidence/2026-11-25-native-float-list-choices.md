# Native model List bounds reported as floating-point values

Studio Pro 8.1.1 build 113103 on macOS arm64 reported an Ampire stomp
selector with a List type and exact floating-point bounds 0.0 to 15.0.
R357 returned `List choices require integer bounds 0..max` on the
existing MCP path. No model change was requested.

The corrected native List reader retains the observed numeric
representation for label formatting. It accepts a zero lower bound,
an integral upper bound and the existing -1 empty-list sentinel.
Fractional, mixed, nonzero-based and out-of-range bounds remain
rejected. List snapshot guards include the formatting representation.

R358 used the patched native source at `ec89417` and the normal
`plugin_parameter_choices` MCP tool. A single bounded Ampire insert
reported and paginated 16 stomp, 6 amplifier and 18 cabinet labels.
The last page and the empty page at each exact end had the expected
offset and cardinality. The labels matched explicit raw formatting
for all 40 entries. Among the stomp labels, Compressor is index 2
and Wah Wah is index 15 on this build.

MCP did not select any model. Separate reads confirmed all 90 stable
non-meter values, the full component fingerprint, mixer, empty
timeline, and insert chain. The owner made 59 requests in 5.030 seconds,
and the host exited normally after 49.164 seconds. The scratch was
discarded and still byte-identical. All 18 managed files and 10 touched
app-profile files were restored.

Local checks passed for 259 server tests, 501 native add-on tests,
148 parameter-library tests, and 4 parameter-library documentation tests.
