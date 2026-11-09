# Exact-device External Effect editor lease

A std-only Rust resolver for an **already open** External Effect editor in the
validated Fender Studio Pro **8.1.1 Build 113103 arm64 / CCL5.1.2** runtime. It
binds the UI controller to the caller's actual native Device identity, without
matching window/channel captions. It performs no opening, clicks, native setter,
arbitrary object path, IPC, installation or dynamic symbol lookup.

The independently authored MIT implementation vendors no SDK implementation,
header or proprietary app resource. Normal Cargo build and portable 64-bit mock
tests require neither a DAW nor an SDK. CCL declaration facts come from official
[`ccl-5.1.2`](https://github.com/cclsoftware/ccl-framework/tree/ad5876b1a7b64aa247f4378c86a7862e7c284ba3),
commit `ad5876b1a7b64aa247f4378c86a7862e7c284ba3`.

## API and caller contract

```toml
[dependencies]
fender-native-editor-api = { path = "../editor-api" }
```

`unsafe resolve_external_owned(expected_device, gui) -> Result<ExternalEditorLease, Error>`
consumes one owned Device IUnknown reference, including on error. Before calling
it, validate the GUI thread and both exact runtime images:

| Image | Mach-O LC_UUID |
| --- | --- |
| Fender Studio Pro 8.1.1 arm64 main | `6B547378-B3BB-322A-87C9-901493FD2BED` |
| Its CCL5.1.2 cclgui framework | `1A8C66AE-250D-3C7F-801C-6904D5082F57` |

`GuiApi` supplies already resolved `get_desktop` (`CCLGetDesktop`) and
`get_constant_string` (`CCLGetConstantString`) pointers from that runtime.
Constants are requested only for the fixed names `DeviceSlot` and `Device`.
Reject other runtime versions before using these public virtual interfaces;
matching IIDs alone cannot establish ABI compatibility.

The caller retains native document/model/service context through the lease's
Drop, serializes requests, prevents reentrant writes, and validates context
before and after native operations. The type is `!Send`/`!Sync`. Construct, use
and destroy it on the verified GUI thread; its creator-thread check is additional
protection. Native exceptions and Rust panics must not unwind through FFI.

The lease exposes:

- `identity()` — Device UID, class UID and canonical/UID match method.
- `controller()` — a borrowed NonNull **IController interface** pointer. It
  remains borrowed; retain it before transferring an owned reference elsewhere.
  Keep the lease alive throughout the entire synchronous request.
- `check()` — bounded re-resolution and verification of the same reachable
  window, editor, Device and External component/controller identities. A stale
  or ambiguous editor fails. This is a point-in-time check, not a lock against
  future host changes and not permission to use the pointer after lease Drop.

For a setter's final guard, run the editor check, then final **borrowed** active
document/model/Device comparisons against retained pins, then the service-active
atomic check. Avoid releasing temporary native owners after those final checks:
release callbacks may reenter termination, document switches or editor changes.
The public parameter helper supplies pre/post dispatch guards and readback;
this resolver itself performs no mutation or automatic restoration.

## Resolution and bounds

The native Device must have a nonempty object UID and one of these exact class
UIDs, observed on the validated 8.1.1 runtime:

| Device | Class UID |
| --- | --- |
| External Effect Mono | `873281D9-58DE-4B2D-B3BD-528F549B1AD9` |
| External Effect Stereo | `889DD9F8-14F0-44A5-BCEE-0F2B157FC5D4` |

Other device classes are explicitly unsupported. Up to **64** desktop windows
are inspected. A candidate requires the exact IObject class `DeviceEditor` and
the fixed IObjectNode route `DeviceSlot/Device`. The returned Device class must
match the expected class. Canonical IUnknown identity is preferred; a distinct
adapter is accepted only when **both nonzero instance UID and class UID** match.
Multiple matching Device editor windows are rejected, even if one otherwise
looks usable.

Within that same window, the source-proven path of child indices
`0/0/0/0/1` must reach an IView controller whose exact native class is
`ExternalEffectComponent` and which supports IController. There is no heuristic
tree fallback. The fixed path has five edges; explicit bounds are 128 inspected
views and depth16. The resolver retains the entire path and window/editor/slot/
Device/component/controller identities. The component's class UID may be empty;
its exact IObject class name and retained canonical identity are used instead.

`check()` re-resolves and compares the pinned identities, releases its validation
temporaries, then rechecks the pinned window's desktop reachability, exact
editor and Device mapping, Device UIDs/classes, each direct parent/child view
relationship, and terminal External controller. Its final relationship phase
uses borrowed getters and existing pins, with no native owner temporaries left
to release after success. Desktop count changes invalidate the result.

The named Device route and External fixed view path are backed by earlier
bounded 8.1.1 observations; the fixed component path was used in both actual
Mono/Stereo raw-value restoration proofs. This reusable exact-Device resolver
has **mock verification only** until separately exercised by the owning add-on
in the app. Do not claim that the earlier caption-guarded proof validated this
new identity binding or all editor layouts.

## Build and optional ABI validation

```sh
cargo +1.88.0 fmt --manifest-path native/editor-api/Cargo.toml -- --check
cargo +1.88.0 test --manifest-path native/editor-api/Cargo.toml
cargo +1.88.0 clippy --manifest-path native/editor-api/Cargo.toml --all-targets -- -D warnings
sh native/editor-api/checks/check_sdk.sh /path/to/separate/ccl-framework-5.1.2
```

The optional command pins the exact official commit and 32 API header hashes,
checks UID/String layouts and compiles all **18** used virtual calls to verify
their arm64 offsets, including IViewChildren's readonly `isChildView` slot12.
Compiler output is temporary; no SDK code is linked or loaded. The compiler
probe uses public declarations for IUnknown, IObject/ITypeInfo, IObjectNode,
IDesktop/IWindow, IView/IViewChildren/IViewIterator and text constants. C++
Variant returns and other unverified private methods are absent.

Mocks cover canonical and stable UID matching, wrong Device with the same class,
class mismatch, unsupported Device, missing/duplicate windows, missing path,
wrong component, detached/reassigned editors, native UID changes, native release
reentry, ownership balance, window limits and wrong-thread rejection. Compile-fail
checks verify that leases cannot be sent or shared across threads. These checks
perform no app, profile, SDK or native library I/O.
