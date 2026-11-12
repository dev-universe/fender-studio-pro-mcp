# Audio and MIDI hardware observations

The native Rust server exposes two local tools on macOS. They do not require an open DAW document.

| Tool | Arguments | Result scope |
|---|---|---|
| `hardware_devices` | Empty object | CoreAudio devices/default IDs and CoreMIDI source/destination metadata |
| `midi_input_observe` | Signed nonzero `source_uid`, `duration_ms`100..5000, `max_events`1..256 | One bounded MIDI1 input projection from a freshly resolved source |

Both reject unknown arguments. Neither accepts a destination, arbitrary native call, script, shell command, path or audio-device selection.

## Standalone commands

After building the server, inventory devices without creating a DAW RuntimeHandle:

```sh
./target/release/fender-studio-pro-mcp hardware-devices
```

Resolve the currently unique online USB MIDI source from that result. Use its signed `unique_id` as `source_uid`; negative values are valid. Do not select the Transport endpoint or the bridge wake source as a substitute for the keyboard USB MIDI source.

```sh
./target/release/fender-studio-pro-mcp midi-input-observe \
  --source-uid SIGNED_UID_FROM_FRESH_INVENTORY \
  --duration-ms 1000 --max-events 64
```

The standalone commands bypass the DAW runtime. The existing MCP `serve` startup still initializes its ordinary bridge wake source; local tool dispatch does not make a DAW request.

## Inventory semantics

Each property is observed or unavailable, with the native status when available. Audio rows include object ID, UID, name, manufacturer, alive state, nominal sample rate, buffer frames and input/output channel counts. Default audio input/output/system-output IDs are separate OS observations. They do not establish which device Studio Pro selected.

Each device domain retains full before/after ID lists, nullable counts and nullable membership equality. These are sequential observations. Equality does not establish an atomic graph or future connection guarantee. Missing metadata is not converted to an empty successful list.

The inventory routine creates no MIDI client, port, input subscription or audio stream. It sends no messages and changes no device settings. Native sizes, counts, strings and buffer-list parsing are bounded. Audio sample-data pointers are not dereferenced.

## Observer lifecycle and limitations

The observer re-resolves one unique online, device-owned source before subscribing and compares its full binding afterward. It decodes bounded explicit-status MIDI1 channel messages. Real-time system bytes are skipped within messages; running status is not accepted. System/SysEx/unsupported data is counted without exporting vendor payloads. Event limits, malformed data and callback contention remain visible.

One observation is admitted at a time in a process. Each input port is explicitly disconnected and disposed, and both native statuses are checked. A cleanup failure poisons further observations. The implementation intentionally retains at most one CoreMIDI observer client until process exit, following the SDK warning about disposing a process's last client. Callback storage has process lifetime and uses never-reused numeric generations. `callback_capture_closed` describes revoked application capture; it does not claim native callback-thread quiescence.

No message is forwarded or sent by this observer, and no audio stream is opened. An empty window can validate subscription/window/cleanup while proving no physical key activity. Even observed MIDI notes identify OS input events; endpoint identity alone does not authenticate a physical performance. DAW routing and audio remain separate results.

## Verified on 2026-11-13

Root tested the source-bound isolated release on macOS Apple Silicon with Volt276 and Keystation88 MK3 connected by USB only.

- Inventory observed5 audio devices,3 MIDI sources and3 destinations with matching sequential before/after memberships. Volt reported2 input and2 output channels at48kHz and512frames. USB MIDI and Transport were distinct Keystation sources.
- Standalone inventory completed in1.3176seconds. A fresh inventory/observer/inventory sequence completed in1.5465seconds; its requested100ms window measured105ms, with0events and successful disconnect/dispose statuses.
- Actual MCP listed45tools, rejected an unknown observer argument, and completed5000ms and100ms requested windows in one server process. Measured windows were5002ms and102ms; both had0events, stable source binding and cleanup statuses0. The MCP test completed in5.846seconds and its server exited normally.
- Full product tests127/127, formatting, strict all-target Clippy and isolated release build passed. Independent exact-function inventory tests17/17 and observer tests29/29 passed. Two initial observer defects were fixed and independently retested: interleaved real-time bytes and embedded-NUL metadata truncation.

These results establish metadata and bounded passive-subscription behavior. Physical key performance, instrument response, audio capture, analog loopback, External Effect ping and CV output isolation remain unverified. USB-only connection provides no physical return cable.

See [current progress](PROGRESS_2026-11-13.md) for actual Studio Pro Audio Setup observations and its temporary-setting restoration obligation. Raw device IDs, serial-bearing UIDs, private songs and runtime receipts stay outside public Git.
