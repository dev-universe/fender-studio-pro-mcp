# Native Fender MCP add-on

A Rust `cdylib` loaded as a normal CCL ProgramService by Fender Studio Pro
**8.1.1 Build113103, macOS arm64**. The MCP process remains Rust; this add-on
provides a finite, synchronous native API to the existing DAW JavaScript bridge.
It includes no vendor implementation, SDK headers, app resources or songs.

The service exposes `Host.Objects.getObjectByName("FenderMcpNative")` after normal
CCL scripting-module attachment. Its two methods return JSON strings:

```js
native.capabilities();
native.request(resolvedDevice, JSON.stringify({
  operation: "parameters", source: "auto", offset: 0, limit: 64
}));
```

The target is the bridge's exact resolved native `Device` object. It is passed
as an object inside the existing serialized GUI/document transaction, never as
a caller-supplied object path. The add-on has no independent IPC, eval, shell,
generic native-method dispatch or arbitrary interface selector.

## Requests

Only these operations and fields are accepted. Unknown and duplicate fields
fail before target access. `expected_instance`, when supplied, is the uppercase
nonzero native object UID returned in a prior response.

| Operation | Fields |
| --- | --- |
| `inspect` | `source`, `expected_instance` |
| `parameters` | `offset` (default0), `limit` (default64,1..128), `source`, `expected_instance` |
| `parameter` | `param`, `source`, `expected_instance` |
| `choices` | `param`, `offset` (default0), `limit` (default64,1..128), `source`, `expected_instance` |
| `set_parameter` | `param`, `mode`, `value`, `source`, `expected_instance` |

`source` is `auto` (default), `dsp` or `controller`. For indexed metadata, `auto`
selects a positive direct Device DSP provider first, then a controller. For value
reads and writes, `auto` selects a controller; explicit `dsp` scalar operations
fail because the private DSP interface is enumeration-only. Controller lookup
uses the actual Device, its normal component alias, or the exact-device External
Effect editor resolver. The External editor must already be open. The add-on
never opens it or matches a device by title, selection or first-window position.

`choices` reads the native labels of one zero-based List parameter without
selecting an item. It uses the same controller resolver and returns indexed
`choices:[{index,text}]`, count, offset and next offset. A failed native label
copy discards the page. Empty native ranges0..−1 return zero choices and make
no label calls. This includes External Effect send/receive selectors when the
host has no configured hardware ports. Positive physical routing still needs
actual user I/O configuration for a live check. DSP metadata alone does not
provide list labels; unsupported providers/kinds fail explicitly.

Setter modes are `raw` (finite JSON number), `normalized` (finite number in0..1),
and `text` (string). Parameter names are bounded to512 UTF-8 bytes and text to4096
UTF-16 units, without NUL. Requests are at most16KiB UTF-8 and replies256KiB.
Each request rejects reentry and checks a two second budget between bounded
native operations; a native call cannot safely be preempted once entered.

Successful replies contain `ok:true`, `instance:{object_uid,class_uid,
native_class,object_id}` and the actual selected `source`. Metadata pages expose
`parameters`, `reported_count`, `offset`, `next_offset`, `page_complete`,
`all_indices_read` and explicit failed indices. `all_indices_read` is true only
when this single call starts at0, reaches the reported end and has no failures.
The bridge must aggregate stable instance/count/index coverage across pages.
A zero DSP count covers that provider, not every UI/sampler/program/layer scope.

Controller descriptions include scalar `value`, `text`, `min`, `max`,
`normalized`, `enabled`, `readOnly`, `writable` and typed native support metadata.
Native i64 values outside ±(2^53−1) use null scalar fields and exact decimal
strings in typed metadata; unsafe integer state/ranges disable all native write
modes. Raw numeric requests outside that magnitude are rejected before dispatch.
Non-finite float bits are hexadecimal strings rather than lossy JSON integers.
Opaque/unsupported values remain null and non-writable. Setters return before
and after descriptions and `verified`; accepted input is not a verified result.
Errors use `{ok:false,error:{code,message},outcome:"not_applied"|"unknown"}`.
A context failure after a possible setter remains unknown and is never retried.

## Guards and ownership

Every request retains the active IDocument and model through canonical IUnknown
references, the exact target and its IObjectNode, and the DocumentManager.
The final setter guard checks the external editor lease first, then uses pinned
borrowed document/model/DeviceUID/classUID reads and an atomic service-active
check. The reviewed parameter helper invokes this guard as its last pre-setter
and first post-setter operation. Native output clone/assignment and any old output-owner release happen before
the final borrowed guard. Exactly one per-service receipt lease retains the
selected provider/editor, document/model and target through callback return.
The next target request captures its new pins before releasing the old receipt,
then checks for any destructor-triggered document/service change before work.
Normal stop drains the receipt under the service reentry guard. This bounded
lease may keep the last closed document/device alive until the next valid target
request or shutdown; it never grows with call count. This complements the bridge's request/session/document guard;
it does not replace cross-process DAW serialization in the MCP owner.

Only the exact executable UUID `6B547378-B3BB-322A-87C9-901493FD2BED` and cclgui
UUID `1A8C66AE-250D-3C7F-801C-6904D5082F57` are supported. Other images fail before
READY or script-object registration. ABI declarations are pinned to official
[CCL5.1.2 commit ad5876b](https://github.com/cclsoftware/ccl-framework/tree/ad5876b1a7b64aa247f4378c86a7862e7c284ba3).
Same-named interfaces in newer SDKs are not accepted as compatible.

The service uses normal `IScriptingManager.startup(ownID,ownModule,null,false)`
and pairs `shutdown(ownModule,false)` after successful object unregistration.
Initialization, stop-during-registration, duplicate ownership and shutdown
reentry retain their tested rollback behavior. Each stopped instance remains
permanently inactive even if a new instance starts. The service has no
process-total call cap.

Public Darwin `RTLD_NOLOAD|RTLD_NODELETE` pins the already loaded **own** image,
and an identity/type-checked own CFBundle is retained until process exit. No
vendor image is opened. These intentional pins protect retained script metadata
and late JS references; they do not excuse leaked service/table/JS references.
Normal exit logs require those references to reach zero. Hot reload is not a
supported contract. Local bounded lifecycle logs contain static event names and
counts, not song titles, parameter values or native addresses.

## Build and verification

```sh
cargo +1.88.0 test --manifest-path native/addon/Cargo.toml
cargo +1.88.0 clippy --manifest-path native/addon/Cargo.toml --all-targets -- -D warnings
sh native/build.sh
```

The build writes the four-file `artifacts/native/fender-studio-pro-mcp.native.mac`
package, a matching packed dSYM and SHA256 manifest. It never installs a profile,
launches the app, modifies security settings or signs vendor code. Rust builds
need no CCL checkout. Optional source-only ABI checks use an external official
CCL5.1.2 checkout:

```sh
sh native/addon/checks/check_sdk.sh /path/to/ccl-framework-5.1.2
```

The check rejects a different commit or any changed pinned public header,
compiles independent layout assertions and verifies arm64 virtual-call offsets.
Compiler probes are never linked into the add-on. Mock tests cover finite
schemas, bounds, opaque values, pinned document/model/target/service guards,
registration reentry, module pairing, ownership and old-proxy inactivity. Two
receipt regressions cover document switches during prior-lease release and
service termination during native output assignment; neither publishes a false
verified outcome.
Actual-host evidence is recorded separately by the project's validation notes;
passing source checks alone does not establish a live parameter or UI outcome.
