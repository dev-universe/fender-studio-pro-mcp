# Native preset catalog library

This std-only crate owns a bounded preset catalog for one already resolved
Studio Pro target. It uses exact public CCL 5.1.2 interface declarations. It
discovers no application, document, slot, service or filesystem on its own.
It writes no preset file and accepts no preset bytes or location string.

## Inputs and ownership

The integrating add-on verifies the supported runtime image identities and GUI
thread, resolves the Device's exact slot and its `Presets` child, and transfers
one owned reference for each `Inputs` field.

| Field | Exact interface | Source of the interface |
| --- | --- | --- |
| `manager` | `IPresetManager` | Fixed host global object table entry. |
| `mediator` | `IPresetMediator` | The resolved Device's owning slot. |
| `data_target` | `IDataTarget` | That same slot's exact `PresetComponent` child. |
| `allocator` | `IClassAllocator` | `System::GetPlugInManager()`, through its inherited allocator interface. |
| `empty_url` | `IUrl` | An independently created, empty native URL supplied by the owner. |
| `file_system` | `IFileSystem` | `System::GetFileSystem()`, through its inherited filesystem interface. |

`discover_owned` consumes all six references on success and failure. The owner
must pass a guard that proves its service, document, Device and slot/Presets
attachments. Every call, foreign callback and final native release must occur
on the validated GUI thread. `Catalog` is neither Send nor Sync.

The attribute factory call is fixed. It uses allocator slot 3 with Attributes
class ID `138ED4E7-1786-4A9D-A0EF-D76FBA49B4DA` and IAttributeList interface ID
`6BB2C332-0137-4820-B137-4701B390E512`. No arbitrary class or interface selector
is exposed. The two source-declared service exports are `CCLGetPlugInManager`
and `CCLGetFileSystem`; the owner must resolve them in the validated host.

The library deliberately requires the empty URL input. The inspected public
SDK exposes no Url class ID. A caller must prove its URL creation path, such
as a fixed in-DAW bridge construction, rather than guessing a private factory.
Each preset location fills an owned clone. The prototype stays empty.

## Catalog and recall

```text
Catalog::discover_owned(inputs, guard) -> Catalog
catalog.page(offset, limit, guard) -> Page
catalog.recall(index, guard) -> Recall
catalog.check(guard)
```

Discovery obtains target metadata from the mediator and calls
`IPresetManager.getPresets`. The returned list owns already opened `IPreset`
objects. The library retains each discovered entry, verifies its `Class:ID`
against current target metadata, and keeps its location private.

The current location policy accepts absolute native `file` URLs with an empty
or `localhost` host and a successful native `fileExists` check. Other protocols,
missing locations, missing files and class mismatches remain explicit entry
statuses. This version does not recall package/resource/network URLs. A
read-only flag is reported as `LocalReadOnly`, not as proof of factory origin.
Preset-manager enumeration may consult its registered file handlers. This
crate does not independently certify those handlers' I/O behavior.

Catalogs contain at most 4096 observed entries and pages contain at most 128.
Reaching the catalog bound reports `catalog_complete:false`; the observed
count is not a claimed total. A missing native preset store is an error, not
an empty complete catalog. Entries contain no public URL or local path.

The owner maps an issued opaque catalog token and index to this object. The
library accepts no external preset object or URL during recall. It checks the
canonical mediator target, fresh target class metadata, selected entry metadata,
location and local availability again before dispatch. It supplies exactly one
retained preset to `canInsertData`, then to `insertData` on the exact Presets
component. The immutable one-entry list and its iterator own their references
if the host retains either interface beyond the call.

The source PresetComponent performs preset notifications and displayed-name
updates. Its `insertData` return only reports accepted input: it can return
true even when preset restoration failed. `Recall.verified` is therefore always
false. A post-dispatch error has `RecallError.dispatched:true` and must remain
an unknown outcome. The caller must independently compare plugin/model state
and editor presentation, and restore its enclosing state capsule as needed.

The add-on must expire catalog tokens on service/document/target changes and
drain them at normal shutdown. Keep a catalog owner through native output
publication. A source/helper test cannot prove any of those owner integrations.

## Inserts folder preflight

`folder::FolderPresetRoute` is a separate read-only precursor. It does not
relax the catalog's Device/class-key rules. Its three owned inputs are the
already bound folder `IUnknown`, the host `IPresetFileRegistry`, and an
`IClassAllocator`. The owner must independently prove the folder's exact
service, active document and unique channel attachment. This library does not
discover or certify that relationship.

```text
FolderPresetRoute::discover_owned(inputs, guard) -> FolderPresetRoute
route.observe(guard) -> FolderObservation
route.check(guard)
```

The route queries only fixed mediator, collector and registry interfaces and
requires their canonical identities to match their sources. It retains the
borrowed handler returned by `getHandlerForTarget` before checking context, then
requires its fixed interface and import/export target flags. It does not use a
default-handler fallback. A changed current handler invalidates the route.

The host mediator fills a newly allocated attribute list. The observation
reports a bounded attribute count and only three optional text attributes,
`Class:Name`, `Class:Vendor` and `Class:Category`. The count is limited to64 and
each text to256 UTF-16 units. Missing fields remain absent. Nontext, oversized,
malformed or changing values prevent a successful observation. These fields
are metadata observations, not a substitute for folder identity or ownership.

Successful String metadata is immediately pinned before any context guard,
including borrowed Variant text. Temporary native owners are released before
the final attachment/guard checks.
The integrating owner keeps the route alive through publication and performs
its fresh context check after any final route release. All calls and releases
remain on the creator GUI thread; the route is neither Send nor Sync. It has
no URL, filename, state token, preset construction, file read/write, store or
restore method. No collection or graph-restoration acceptance is implied.

Failed QueryInterface or factory results never claim or release a nonnull
output. Only exact success plus a nonnull pointer transfers ownership. The
shared catalog helpers use the same rule.

## Verification and provenance

```sh
cargo +1.88.0 fmt --manifest-path native/preset-api/Cargo.toml -- --check
cargo +1.88.0 test --manifest-path native/preset-api/Cargo.toml
cargo +1.88.0 clippy --manifest-path native/preset-api/Cargo.toml --all-targets -- -D warnings
sh native/preset-api/checks/check_sdk.sh /path/to/ccl-framework-5.1.2
```

Compiler-only probes verify 38 virtual-call offsets, two service exports,
UID/String/CString/Variant layouts and 53 exact public source hashes. Folder
preflight also verifies four fixed interface IDs, the import/export flag values
and the exact success/no-interface result constants. They use
an external official SDK at ccl-5.1.2 commit
`ad5876b1a7b64aa247f4378c86a7862e7c284ba3` and are never linked into the library.
The Rust implementation and probe calls are independently authored. No CCL
header, implementation or proprietary resource is included in this repository.

Mocks check local filtering, class and instance replacement, selected-entry
identity, data-target refusal, uncertain accepted loads, context loss,
destructor reentry, retained list ownership and bounded catalog pagination.
These checks do not establish actual-host preset recall. The owner must run
the resulting add-on in the real supported DAW.
