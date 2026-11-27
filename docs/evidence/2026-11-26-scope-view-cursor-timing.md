# Scope cursor timing and the display projection

R525 re-extracted the installed `studioeffects` arm64 image from
Studio Pro 8.1.1 build 113103. The UUID and SHA-256 matched the
[earlier capture and display study](2026-11-26-scope-capture-display.md).
REA and Ghidra traced the separately registered `ScopeView` class
and its cursor path. Craft-Anything 0.6.1 independently read seven
20-byte windows, all matching their bytes in the same Mach-O file.

| Route | Observation |
| --- | --- |
| View binding | The `ScopeView` registration constructs the view through `0x45674`. The constructor at `0x456d0` installs separate primary and secondary view tables. The attachment at `0x45904` retains the display object. |
| Cursor input | The primary view table entry at `0x509840` resolves to `0x468a4`. Its cursor update uses the current point and a stored reference point. It consults CCL system ticks in the view refresh and formatting path. |
| Units and display width | The branch for `Difference:` reads the display width from the retained header and relates cursor positions to the view bounds and configured scale. The packed constructor constant independently decoded as four display channels and a width of 800. The formatter can present samples or seconds and milliseconds. |
| Secondary query | The constructor attaches the secondary table at `0x509870`. Its first method either returns one of two subobjects of the same view with a retained owner or delegates to the common query routine. Neither observed successful branch returns the display pointer or the Scope DSP acquisition context. |

The cursor's units describe the geometry of the configured display.
The observed cursor path does not join the renderer to a producer
frame, trigger generation, sample fill count or immutable copy of
the DSP window. The inspected query and cursor paths therefore leave
direct start and completion timing and a coherent sample read open.
This trial was offline and did not exercise the host, its audio
graph or a capture-state edit.
