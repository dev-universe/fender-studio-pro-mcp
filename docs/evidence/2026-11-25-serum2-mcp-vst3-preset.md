# Direct Serum 2 VST3 preset application

R319 ran in Studio Pro 8.1.1 build 113103 on macOS arm64. The two exact
classes came from the Apple-notarized Xfer-signed Serum 2 2.1.5 VST3, in demo
mode. The executable mapped in the Studio Pro process matched the staged
signature and digest. The scratch song had a Music track for Serum 2 and a
stereo audio channel with Serum 2 FX in its first insert slot. The transport
was stopped, the timeline was empty, and Main was set to zero.

For each instance, `serum2_preset_apply` applied a synthetic processor-only
VST3 preset carrier from the Ableton MCP experiment. The caller supplied the
current document, exact target and instance, and SHA-256. The native receipt
was `applied_unverified` and matched the requested instance and file digest.
The server removed the staged copy before independent DSP reads. All 2,623
typed parameter rows were read after each apply. Serum 2 changed 15 values,
and the editor displayed a 150 ms envelope release. Serum 2 FX changed 13
values, and the editor showed Distortion followed by Equalizer. The peer's full typed
values agreed with its pre-apply snapshot after each operation.

The R318 Studio Pro file-menu trial and R319 MCP trial changed the same sets
of parameter tags. Twenty-six of the 28 changed values matched by product.
Both trials reported raw `Scale` values 51 on the first loaded instance and
52 on the second. R318 loaded FX first. R319 loaded the instrument first.
The two inputs encode an inactive, unnamed pitch quantizer. Both editors
showed the scale selector inactive after R319. The `Scale` index is recorded
as load-order dependent for this experiment. The editor's internal preset
display remained `- Init -`, and the Studio Pro header used the staged file
name. The measurement covers processor application. Editable save and reload
remain unverified with the official demo.

An incorrect document token, changed file digest, wrong VST3 class, and
peer instance ID were refused for each target. All four refusals per target
preserved the original typed values. A separate delayed instrument read
repeated all 15 observed changes. The final timeline stayed empty.

Initial binding and scalar checks, application, independent reads, and
editor checks used 397 requests in 49.338 seconds of measured API work. The
scratch host exited normally after 259.251 seconds. The song was closed
without saving. All 18 managed MCP files and the original scratch file were
rehashed after restoring the normal installation, and ten Studio Pro
configuration and cache files were restored. The temporary plug-in copy was
removed from the scan directory.
