# VST3 output refresh and edit-controller call

R457 traced the exact Studio Pro 8.1.1 build 113103 macOS arm64
executables from [R454](https://github.com/dev-universe/fender-studio-pro-mcp/pull/245).
REA and Ghidra 12.1.4 followed 22 functions. Craft Anything 0.6.1
independently matched 13 ranges. Ten loader rebases, 28 instructions and
the CCL parameter-provider interface identifier matched across both
installed images. All work was offline.

The current hosted effect owns the parameter folder and retains the CCL
parameter-provider interface belonging to the same hosted component.
The output-driven refresh can first read the current edit-controller
value through VST3 `getParamNormalized`. This path translates the host
parameter ID to the VST3 ID and converts the value to Float32. The host
compares that value with the current parameter proxy value. When they
differ, the same provider can call the same edit controller's
`setParamNormalized` with the translated ID and the host value converted
back to Float64. The caller returns without recording or branching on
the setter result. The original processor value and comparison inputs can
differ after the intervening host conversion.

Stock folder notifications can carry a host parameter ID and value in
selected paths. Another publisher uses the same event name with no
arguments. The observed stock listener uses the ID to refresh a binding.
The terminal setter call has no associated notification that identifies
the originating output point or publishes the controller result. The
VST3 controller contract also excludes echoing host-set values through
the component handler. An MCP read can independently observe the settled
proxy state for an exact instance.
[Steinberg IEditController](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IEditController.html)

The seven [FabFilter legacy cases](2026-11-24-fabfilter-legacy-readback.json)
still lack positive stopped-host retention evidence. The six isolated SDK
zero points and the single Pro-G negative point remain independent
observations. The isolated Pro-G driver skipped delivery of the negative
point, and the settled host zero alone does not locate where the value
changed. No MCP control or host observer was added.
