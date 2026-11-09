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
| `inspect` | `source`, `expected_instance`, optional `include_nodes`, finite `node_scope` of `target`, `graph` or `plugin_controller` |
| `model_catalog` | finite `catalog` of `categories` or `effects`, `offset`0..4096, `limit`1..32 (default32), required `expected_instance` |
| `parameters` | `offset` (default0), `limit` (default64,1..128), `source`, `expected_instance` |
| `parameter` | `param`, `source`, `expected_instance` |
| `parameter_details` | `param`, required explicit `source` of `dsp` or `controller`, required `expected_instance` |
| `format_values` | `param`, required explicit `source` of `dsp` or `controller`, `values` with 1..32 finite numbers, required `expected_instance` |
| `choices` | `param`, `offset` (default0), `limit` (default64,1..128), `source`, `expected_instance` |
| `set_parameter` | `param`, `mode`, `value`, `source`, `expected_instance` |
| `parameter_action` | `action` of `swap_filter_frequencies`, required `expected_instance`; fixed direct DSP source |
| `state_capture` | required `expected_instance` |
| `state_restore` | issued `token`, required `expected_instance` |
| `state_release` | issued `token`, required `expected_instance` |
| `presets` | `offset` (default0), `limit` (default32,1..64), optional issued `catalog_token`, required `expected_instance` |
| `preset_apply` | issued `catalog_token`, issued `index`, required `expected_instance` |
| `preset_release` | issued `catalog_token`, required `expected_instance` |
| `inspect_aux` | required `expected_instance`; read-only auxiliary reflection |

Initial `presets` and `inspect_aux` take exactly three arguments:
`request(resolvedDevice, boundedJson, auxiliaryObject)`. For initial presets,
the bridge supplies its fixed `new Host.Url("")` prototype; the add-on queries
the public IUrl interface and the helper rejects a nonempty URL. JSON never
accepts a URL, path, native object, factory selector or imported preset bytes.
Catalog continuation, apply/release and all other operations require the
existing two arguments. The optional scripting metadata does not relax these
operation-specific arity checks.

`presets` is restricted to the observed Device -> owning slot -> Presets route.
The slot's exact class must be `AudioInsertSlot`, `AudioSynthSlot`, `MusicFXSlot`
or `AudioFXSlot`, and its Presets child must be `PresetComponent`. The class names
are retained and rechecked alongside canonical pins and node identities,
including slots with no class UID. Other slot families remain unsupported.
Class/reflection observations and temporary native-owner releases precede
fresh borrowed UID and parent/Presets-child checks, with the current document/target
check last. These are sequential observations, not an atomic graph snapshot.
Actual 8.1.1 catalog checks passed for `AudioFXSlot` on Surround Panner (zero
current entries) and Console Shaper (13), including released-token refusal and
six independent complete typed provider confirmations per fixture. Two Console
Shaper recalls also changed writable values and restored the baseline. An empty
current panner catalog establishes no preset-recall result. A stable slot object UID is still mandatory;
missing object identity is rejected even when the class name is allowed.
It lists at most4096 entries per catalog and retains at most four catalogs per
service. Each random64-uppercase-hex token pins the canonical document, model,
Device, owning slot and Presets child, including their observed node identities.
The helper additionally pins the mediator target and metadata class. Every
native callback and publication guard rechecks these bindings. The next target
request expires catalogs from another document/model; normal stop drains all
catalog and receipt owners. The normal-exit diagnostic includes
`preset_catalogs:0` in its clean predicate.

Pages return `entries:[{index,name,class_key,read_only,availability,recallable}]`,
`observed_count`, `offset`, `next` and `catalog_complete`. Only indexes issued by
this catalog can be recalled. Matching-class presets backed by existing native
local files are recallable; unavailable, remote, package/resource and mismatched
entries remain explicitly excluded. `excluded_on_page` reports reason counts
without paths or URL strings. Read-only does not establish factory origin.
Recall uses the fixed Presets IDataTarget and returns `accepted` separately
from `verified:false`: acceptance yields `applied_unverified`, never a sound or
state verification claim. Loss after dispatch yields `unknown` with no retry.
Release confirms token removal only; a publication lease retains the catalog
until the next target request or stop, so destructors cannot follow a success
guard. A stale mediator can be released for its original document/model/Device
without authorizing another recall.

`inspect_aux` observes IObject/ITypeInfo on one bridge-created auxiliary object,
with one node, zero depth and zero ancestors. It does not require IObjectNode.
The reply labels this `auxiliary_type_observation` and
`mutation_capability:false`; the metadata does not authorize later invocation.
Its own retained Inspection and the root Device Context survive publication.
No auxiliary method, property setter, object factory or arbitrary API is called.

`inspect` with `include_nodes:true` and `node_scope:plugin_controller` observes
the wrapped plugin's declared `IControllerProvider` controller for the exact
Mustang Native or Rumble Native Device. This scope requires `expected_instance`.
It retains the Device parent, alias, plugin, provider and returned controller
through publication. Node/type inspection is bounded to96 nodes, depth5,
zero ancestors and32 children per node. A stable null controller returns
`controller:null`, `complete:false` and a `controller_absent` diagnostic.
Missing Node or reflection interfaces remain explicit observations.

This report has `scope:alias_declared_controller_observation`,
`mutation_capability:false` and `atomic_snapshot:false`. Fixed QueryInterface
views and canonical identities are checked at guarded observations before final
context guards and borrowed attachment checks. They do not certify a
continuously current GUI graph, a model catalog or any subsequent mutation.
Future model selection must acquire and revalidate its own binding. Callers
cannot choose native paths, methods or model names through this inspection.

`model_catalog` uses its own retained fixed
`workspaceroot/plugInUnknown/signalchain/effectSelector` binding for those same
two observed Device classes. `categories` maps to `categoryModel`; `effects`
maps to `effectListModel`. The selector's public `IController.getObject` result
is borrowed, retained immediately and independently queried for `IItemModel`.
Only flat item count and title reads are allowed. No selection, focus, open,
view attachment, preview or confirmation is performed, and no token is issued.

Responses return `catalog`, `model_name`, `count`, `offset`,
`rows:[{index,title,title_available}]`, `next_offset`, `complete` and
`all_flat_rows_observed`. A false title result remains explicit, even when the
producer wrote text. A missing route/model is an error; an observed zero count
is an empty current flat model. Count is limited to4096, pages to32 rows, titles
to512 UTF-16 units and aggregate title text to16384 units. `complete` covers the
requested page's titles; `all_flat_rows_observed` additionally requires offset0
and every observed flat row. Neither claims the union of all model categories.

The reply labels `scope:current_selector_flat_model`, `atomic_snapshot:false`
and `mutation_capability:false`. Rows and count are reread to reject detected
content changes; canonical interfaces, each fixed attachment and named model
are checked after existing context/provider/instance guards. All native owners
remain pinned through output and the receipt. A changed attachment/content
removes catalog rows/count from the failed response. Stable identity and two
matching reads cannot certify an atomic snapshot or a future selection.

`source` is `auto` (default), `dsp` or `controller`. For indexed metadata, `auto`
selects a positive direct Device DSP provider first, then a controller. For value
reads and writes, `auto` selects a controller. Explicit `dsp` scalar operations
resolve each canonical name through the direct Device's public IController.
This lookup accepts an indexed controller count of zero and never falls back to
an alias or editor. The private DSP interface still supplies metadata only.
The separate `controller` lookup uses the actual Device, its normal component
alias, or the exact-device External Effect editor resolver. The External editor
must already be open. The add-on never opens it or matches a device by title,
selection or first-window position.

`parameter_action` accepts only the source-backed Swap Filter Frequencies button
on the exact registered Expander, Gate and Compressor classes. The caller cannot
supply a parameter, value, source or native method. It sends the native button
sequence of maximum with notification, then minimum without notification. The
same parameter and its controller association remain pinned through publication.
The reply reports each dispatch and reset observation separately. Success is
always `applied_unverified`, with `verified:false` and `effects_verified:false`.
Independent reads of the two reported `effect_parameters` and the full provider
must establish the actual swap and any reversal. A lost context, changed binding
or owner, or failed reset after upper dispatch yields `unknown`; it never retries
the pulse or completes a reset after an earlier failure.

`parameter_details` observes one named parameter and its bounded original chain.
The response includes the current `parameter` and `details`, plus
`original_chain.parameters` with separate descriptions and details for each
observed representation. Descriptions retain typed `native_value`, `native_min`,
`native_max`, and `native_default`. Details report precision, five native state
flags, and whether read-only IListParameter QI succeeds. Precision is an
increment/decrement setting. These facts do not establish a model choice count.

Entry zero is the resolved parameter. At most eight distinct canonical
parameters are observed. `original_chain.end.kind` is `self_reference`,
`missing_original`, `cycle`, or `limit`. A cycle includes the earlier entry's
`index`. A limit means the eighth entry was not followed. Original pointers are
retained before context callbacks, deduplicated through pinned IUnknown
identity, and released before accepting the observations. The existing
provider/document receipt survives output publication. Observations are
sequential and expose no original handles, label enumeration, or write route.

`format_values` formats explicit raw numeric values through the public
`IParameter::getString` getter. It preserves order and duplicates. JSON inputs
must be within the safe numeric magnitude of 2^53-1, within the observed native
range, and losslessly convertible to the current Integer or Float carrier.
The response has `values` with `requested_value`, exact typed
`requested_native_value`, the actual typed `native_value`, and observed `text`.
It also has `before`, `after`, and `current_unchanged:true`.

The helper rejects any metadata, precision, state, exact raw-value, or normalized
change between observations. Native outputs and the resolved parameter are
released between context guards before success. The provider/document receipt
survives publication. A copied label is a string representation of the supplied
value. It does not establish a model domain, choice count, or selection.
`getString` returns void, so an empty output remains an observed empty string
without a claim that the provider accepted the value. No parser, setter, or
update method is called.

`choices` reads the native labels of one zero-based List parameter without
selecting an item. It uses the selected source's named resolver and returns indexed
`choices:[{index,text}]`, count, offset and next offset. A failed native label
copy discards the page. Empty native ranges0..−1 return zero choices and make
no label calls. This includes External Effect send/receive selectors when the
host has no configured hardware ports. Positive physical routing still needs
actual user I/O configuration for a live check. DSP choices use the direct Device
IParameter, not the private DSP metadata ABI. Unsupported kinds and floating-scale
List ranges fail explicitly.

Setter modes are `raw` (finite JSON number), `normalized` (finite number in0..1),
`text` (string), and `float_bits` (exactly 16 hexadecimal characters encoding one
finite native f64). The bit mode requires the current native value to be a Float
and retains the same range, enabled/read-only and context guards. It refuses
integer and opaque values. It preserves negative zero and finite values whose
JSON number representation changes during the bridge roundtrip. Parameter names are bounded to512 UTF-8 bytes and text to4096
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

DSP records retain their canonical index, name, tag and uninterpreted flags.
Each also carries `native_description` from the direct Device's named parameter.
Its local name/tag remain inside that description and do not replace the DSP key.
A failed access description returns null and an explicit `access_error` on that
record. `access_metadata_complete` and `failed_access_indices` describe this
separate access coverage; metadata `page_complete` does not imply access coverage.

`inspect` includes read-only `capabilities` for the exact Device, its ordinary
component alias, its immediate native parent and the parent's fixed `Presets`
child. Each present object reports its native class, available node identity and
fixed IStorable, IPresetMediator, IPresetMetaDataProvider and IDataTarget queries.
An unavailable node identity stays explicit. A separate boolean reports whether
the global object table's exact IPresetManager service can be queried. These
interface observations do not prove that save/load or preset recall works.

Controller descriptions include scalar `value`, `text`, `min`, `max`,
`normalized`, `enabled`, `readOnly`, `writable` and typed native support metadata.
Native i64 values outside ±(2^53−1) use null scalar fields and exact decimal
strings in typed metadata; unsafe integer state/ranges disable all native write
modes. Raw numeric requests outside that magnitude are rejected before dispatch.
Non-finite float bits are hexadecimal strings rather than lossy JSON integers.
Finite native floats also include their exact `bits_hex`, preserving signed zero
when a JSON client normalizes the scalar number.
Opaque/unsupported values remain null and non-writable. Setters return before
and after descriptions and `verified`; accepted input is not a verified result.
Errors use `{ok:false,error:{code,message},outcome:"not_applied"|"unknown"}`.
A context failure after a possible setter remains unknown and is never retried.

## Bounded state capsules

State operations use the resolved Device's normal `IComponentAlias` and its
`IStorable` interface. They use no editor selection, caller-provided path or
imported state bytes. A capture saves into an independently owned, seekable
Rust stream and returns a random 64-character uppercase hexadecimal token,
byte count and SHA256. The raw bytes remain inside the native service.

Each service holds at most 8 capsules, 16 MiB each and 64 MiB total. Tokens bind
the canonical document and model, Device identity and component alias. A different
service cannot redeem them. The next valid native target request expires
capsules belonging to an older document or model, and normal stop drains all
capsules. A retained native stream is sealed and its bytes removed when the
operation finishes, so late callbacks cannot modify the saved copy. Stream
overflow, invalid seeks and writes to restore input produce sticky failures.

Restore accepts only a token issued by the same service. It checks the alias
and parent attachment immediately around `IStorable.load`, followed by the
existing pinned document guard. Success returns `load_accepted:true`,
`verified:false` and `outcome:"applied_unverified"`. Independent parameter,
model and graph readback must establish semantic restoration. A failure after
load dispatch remains unknown. The operation never retries automatically.
An issued token remains reusable while its original binding is current; release
removes it. No file type or C++ return-by-value adapter is called.

This API provides an opaque native storage checkpoint. It does not claim that
every built-in supports storage or that a successful load restores every UI,
preset label, graph relation or audible state. Those claims require actual-host
acceptance. SHA256 uses the RustCrypto `sha2` crate; no vendor storage code or
library is linked into the add-on.

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
Compiler probes are never linked into the add-on. Mock tests cover direct Device
named lookup with zero indexed parameters and no alias fallback, canonical/local
parameter identities, fixed capability queries, finite
schemas, bounds, opaque values, pinned document/model/target/service guards,
registration reentry, module pairing, ownership and old-proxy inactivity. Two
receipt regressions cover document switches during prior-lease release and
service termination during native output assignment; neither publishes a false
verified outcome.
Actual-host evidence is recorded separately by the project's validation notes;
passing source checks alone does not establish a live parameter or UI outcome.
