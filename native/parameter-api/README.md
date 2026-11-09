# Native parameter API

A std-only Rust library for owned CCL **5.1.2** parameter data access. It is a
component for the optional native add-on, with no resolver, loader, IPC, dynamic
evaluator or host installation of its own. An ordinary Cargo build and all mock
tests run without Fender Studio Pro, CCL libraries, C++ tools or an SDK checkout.

The Rust code is independently authored and MIT licensed. No CCL source/header
is vendored or linked. [ABI.md](ABI.md) records the source declarations and the
separate compiler checks. See [the native integration contract](../README.md)
before using this library in a DAW process.

## Build and test

From the repository root:

```sh
cargo +1.88.0 fmt --manifest-path native/parameter-api/Cargo.toml -- --check
cargo +1.88.0 test --manifest-path native/parameter-api/Cargo.toml
cargo +1.88.0 clippy --manifest-path native/parameter-api/Cargo.toml --all-targets -- -D warnings
```

The tests use local fake vtables and perform no native loading or application
I/O. They exercise numeric boundaries, readonly/disabled states, Unicode
ownership, false OUT returns, formatter previews, normalization quantization,
and reentrant context loss. Compile-fail documentation tests verify the wrapper
is neither Send nor Sync. These tests are portable to 64-bit Rust hosts; the
unsafe runtime interface contract is specific to the validated macOS ABI.

## Integration

For an add-on crate alongside this one:

```toml
[dependencies]
fender-native-parameter-api = { path = "../parameter-api" }
```

Transfer **one owned IParameter interface reference** into
`unsafe RawParameter::from_owned(pointer, text_api)`. The caller must first
validate the exact runtime ABI, verify the GUI thread, resolve the intended
target, and retain the document/model/editor context throughout use and Drop.
An IUnknown or IObject pointer is not an IParameter interface pointer. The
constructor neither resolves nor queries the target interface. Do not pass a
borrowed reference without retaining it first.

`TextApi` provides already resolved public `CCLGetEmptyString` and
`CCLGetConstantCString` pointers from that same runtime. The helper does not
resolve symbols or load native libraries. It retains the borrowed empty
IString, clones before assigning input UTF-16, and uses only four fixed CString
property identifiers: `value`, `min`, `max`, `default`.

Construct, use and destroy the wrapper on the validated GUI thread. The creator
thread check is extra protection, not proof of the GUI thread. `RawParameter`
is `!Send` and `!Sync`, and Drop releases its owned reference. The runtime and
context must remain valid until that release finishes. Native exceptions or
Rust panics must not unwind across an FFI boundary.

## Operations and results

`describe()` returns the outer native name, tag, type, enabled state, readonly
state, optional raw value/min/max/default, normalized value, display text and
supported operations. It preserves the outer alias identity rather than reading
the underlying IObject `name` property. Values come from IObject Variant **OUT
references**, avoiding C++ Variant BY-VALUE returns. Shared String/Object outputs
are owned and released on all paths, including a native false result. Objects
are exposed only as `OpaqueObject { present }`; nonfinite floats are represented
as bits. Missing properties remain `None`.

| Method | Accepted data | Readback |
| --- | --- | --- |
| `set_raw_number` | Toggle, Integer, Float, List index, Scroll; finite values within the native numeric range, with lossless coercion to the existing representation | Exact raw integer or f64 bits |
| `set_normalized` | The same numeric kinds; finite 0..1 and finite current normalization | Normalized match within 1e-6, with observed raw value |
| `set_display_text` | String; scalar numeric with a stateless formatter; plain numeric syntax when no formatter exists | Exact raw text/number or the formatter's normalized result |

`list_choices(offset, limit, guard)` reads labels for a **List** parameter
without selecting any entry. It returns `ListChoices { count, offset, next,
choices }`, with `ListChoice { index, label }` records. Limits are 1..128 per
page and at most 1,000,000 total choices. Native integer bounds must be `0..max`;
the empty range `0..-1` returns count0 with no label calls. This also works when
the current selection is -1, or the parameter is disabled/read-only. An offset
equal to count returns an empty final page; larger offsets are rejected.

The helper calls only public `IParameter::getString(String OUT, Variant IN)`
at CCL5.1.2 slot43 with a numeric index. It does not call a setter, retrieve
IList selected values, use a Variant BY-VALUE return or guess labels through
`fromString`. Names/tags/types and the choice range are rechecked before each
label and at the end. Context guards surround the call, strict UTF-16 copy and
all native temporary cleanup. Any copy/context/range failure discards the
entire page; no partial page is accepted. Equal counts cannot prove that the
provider kept every label unchanged between observations or pages.

An unselected dynamic List may expose raw integer -1 while its valid bounds
have become integer `0..max` with max>=0. That one case may initialize a valid
in-range index using the numeric setters. Writing -1 remains rejected, and the
empty `0..-1` list stays nonwritable. The exception does not apply to Float,
Toggle, other kinds, float -1.0, min!=0, or another out-of-range current value.
It never applies to post-write verification: the observed raw List index must
actually be within min..max. A single-entry List still at -1 cannot be verified
merely because its normalized getter returns 0 after a refused/no-op setter.
Hardware routing choices cannot be selected or validated when the host exposes
an empty port list; no missing ports or routing success are inferred.

Every setter validates enabled/readonly/type/range and takes a caller context
guard `FnMut() -> Result<(), String>`. The guard must check current active
document/model, target identity, authorization and service running state. It
must reject reentrant termination or replacement, and must not itself change
the parameter. The caller serializes operations and prevents reentrant writes;
this data helper implements no global owner.

The guard runs initially, immediately before the setter, immediately after it,
after releasing native input/formatter temporaries, and after readback. Release
callbacks can reenter, so all such temporaries are released before the final
success guard. Context loss during cleanup prevents any stale readback. The
formatter interface and exact flags used for a display preview are checked
against the refreshed description and again immediately before dispatch.

Errors before dispatch return `Outcome::NotApplied`; context/read failures after
dispatch return `Outcome::Unknown`. There is no automatic rollback. Never
blindly restore a stale target. A successful `MutationReadback` still exposes
`verified`: native quantization may produce `Ok` with `verified == false` and an
observed `after` snapshot. Verification covers this parameter at observation
time, not wider controller side effects or continued validity after the call.

Stateful display formatters and List label writes are explicitly unsupported.
A List's raw value is its index, not its selected label/value. Without a
formatter, units are never guessed. Command, Color, Image, Segments, TextModel
and unknown types have no setters here. A supported data type does not authorize
an arbitrary production target; the integrating service still applies its
target and operation policy.

Input text is limited to 4096 UTF-16 units with no embedded NUL. Output checks
copy status, length, NUL termination, UTF-16 validity and length stability.
Parameter names are limited to 512 UTF-8 bytes. Bounds cause an error rather
than truncation.

## Optional source ABI checks

Obtain the official SDK separately and explicitly provide its path:

```sh
sh native/parameter-api/checks/check_sdk.sh /path/to/ccl-framework-5.1.2
python3 native/parameter-api/checks/regression.py /path/to/ccl-framework-5.2
```

The first command verifies official `ccl-5.1.2` commit
`ad5876b1a7b64aa247f4378c86a7862e7c284ba3`, 32 header hashes, C++ scalar/reference
and layout assertions, and 23 arm64 virtual dispatch offsets. It needs Apple
Clang and Python. Temporary compiler output is removed automatically. The
second command confirms that a separately obtained newer SDK is rejected by
both the version pin and compiled call map. Neither command links the SDK,
loads a native library or runs a DAW.

CCL5.2 inserted two IParameter methods while retaining its IID; matching the IID
alone is insufficient. Source/compiler ABI checks do not establish compatibility
with arbitrary installed binaries. The owning add-on must validate the known
host and CCL UUIDs before constructing this wrapper. Generic string/formatter
paths still require actual app validation; mock success is not a runtime claim.
