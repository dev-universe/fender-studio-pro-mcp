# Scope capture data and the waveform display on Studio Pro 8.1.1

R420 used the arm64 slice of the installed `studioeffects` image from Studio Pro
8.1.1 build 113103. The slice was re-extracted and byte-compared to R333. Its
Mach-O UUID is `CC2A5FBD-96A4-3231-9D30-DB782E03BE20` and SHA-256 is
`14eb3a7bc2a37b7867fcf68fbf1acd6537f4a3bd3431ce383fe26317e7801dca`.
REA 6.9.0 with Ghidra 12.1.4 supplied procedure boundaries, pseudocode and
incoming references. ARM64 disassembly and dyld fixups resolved the relevant
virtual call targets. Craft Anything 0.6.1 Deep independently decoded the
binding, acquisition, display and window-configuration routines.

## Ownership and observation

| Path | Static observation |
| --- | --- |
| Display allocation | The Scope component allocates a retained display object at `0x33e20–0x33eb0`. Its header holds four channels and 800 display values per channel. It allocates four arrays of 800 float-sized values. |
| DSP attachment | The TestScope component constructor calls `0x36ccc` at `0x3b780`. That routine passes the component's display object to the DSP interface at `0x36d04`. The DSP constructor installs that interface at `0x321c4`. Its first slot at `0x5067d0` resolves to `0x33b0c`, which reaches the retained binding at `0x7030`. |
| View attachment | The editor and MicroDisplay both pass the same component-held display object to `0x45904`, at `0x375d4` and `0x37640`. The ScopeView keeps a reference to that object and separately allocates per-channel view settings. |
| Audio processing | The Scope audio-block function at `0x33414` supplies three channel buffers, then calls acquisition at `0x3350c`. Acquisition sets the trigger flag at `0x7594` or `0x7650`, clamps the appended count to the remaining window and advances it at `0x7aa8`. In Oneshot with the trigger already set, a subsequent invocation checks that the count has reached the limit and takes the hold path at `0x752c–0x7554`. |
| Display updates | After each appended batch, `0x7ab0–0x7ac0` sets and consumes a separate display-refresh flag. The refresh updates the same shared display arrays through `0x7ba8`, `0x7c1c`, `0x7c98` and `0x7cbc`. Reset can also dirty the display. |
| Rendering | The registered ScopeView paint function is reached through the rebased vtable slot at `0x509830`. It reads display length at `0x46200–0x46204` and loads channel values from the shared arrays at `0x462c0–0x462cc`. The trigger guide's index is written by a view setter using controller values. The inspected path supplies neither the trigger flag nor the collected-sample count to the view. |

The refresh flag indicates changes to display data, including batches before the
full capture window. The ScopeView binding establishes object lifetime. The
investigated renderer and producer contain ordinary loads and stores around the
arrays. No additional reader synchronization or snapshot contract was established.
The shipped hardware remote page maps the configured Oneshot parameter and
Retrigger action. A separate acquisition-progress value remains unobserved in the inspected class
routes, renderer and parameter getter.

These results explain why a single new trace cannot establish when acquisition
finished. R380B independently measured a completed visible trace replacement.
R381 verified that the component capsule had the same bytes for distinct held
traces and that a restore cleared the visible hold. R420 used static analysis of the installed image.

## Next live measurement

The declared `timeScale` range is 0.1 through 100. Scope's window calculation
at `0x7138` clamps the scale and multiplies it by the current 800-value display
width. This gives a maximum 80,000 input samples for this display construction,
about 1.67 seconds at 48 kHz or 1.81 seconds at 44.1 kHz. Verify the actual
sample rate and formatted Time Scale in the owned scratch before using this
estimate.

An independent timed UI capture can test whether the visible trace progressively
fills. Use a verified long-window setting with the previously qualified muted,
empty, exact-instance Scope and Tone Generator route. Verify a flat waiting
frame with a threshold above the low input and the Tone Generator output off.
Use one oscillator output transition to a qualifying low-frequency tone such as
40 Hz, and acquire timestamped screenshots of the same locked viewport around
that transition. Compare nonzero coverage in separate waveform regions, looking
for a new prefix before a later full-width trace. Verify the final trace remains
unchanged when the source frequency is changed and then switched off. Use the
timed MCP command and observed frames to bound the change in the renderer.
Missed intermediate frames remain inconclusive. The internal trigger and
completion timestamps and exact hidden audio buffer still require a separate
framework-owned observation.
