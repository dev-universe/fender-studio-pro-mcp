# Read-only node metadata

This std-only crate reads node relationships and compiled reflection metadata
from an already resolved, owned native object. It targets the reviewed macOS
arm64 CCL 5.1.2 ABI. It has no loader, IPC, evaluator, object instantiation,
parameter write, property write or method invocation operation.

The caller verifies the exact host images and GUI thread, retains its active
document/model/target lease, and supplies a context guard. `NodeInspector` takes
one owned reference and is neither Send nor Sync. The returned `Inspection`
retains discovered node identities and attachment owners. Temporary reflection
and String owners are released before a final guard. Keep the inspection in
the native completed-receipt lease through publication. Call its `check` and
then the outer document/target guard immediately before publishing. Drain old
inspection leases on the same GUI thread, within the owner's existing cleanup
guard.

```rust,no_run
use fender_native_node_api::{Limits, NodeInspector, TextApi};
use std::{ffi::{c_char, c_void}, ptr::NonNull};

unsafe fn inspect(
    owned_target: NonNull<c_void>,
    get_constant_string: unsafe extern "C" fn(*const c_char) -> *const c_void,
    mut guard: impl FnMut() -> Result<(), String>,
) -> Result<fender_native_node_api::Inspection, Box<dyn std::error::Error>> {
    let inspector = unsafe { NodeInspector::from_owned(owned_target, TextApi { get_constant_string }) };
    let inspection = inspector.inspect(Limits::default(), &mut guard)?;
    drop(inspector);
    inspection.check(&mut guard)?;
    guard()?;
    Ok(inspection)
}
```

The report contains indexed Rust records and typed edges, with no native
addresses. Node IDs, optional object/class UIDs and inherited type metadata are
observations. Empty native UIDs remain absent. They are not fabricated from
titles or class names. Canonical IUnknown identities are retained only inside
one inspection for deduplication and cycle detection.

Exploration includes bounded direct children, bounded parents, and these fixed
delegate names: NodeController, Slot, Device, Graph, Combinator, Presets,
RoutingModel and Inserts. These names come from the existing exact SDK or
shipped graph bindings. A fixed lookup's presence does not establish its role
or mutability. The caller cannot supply an arbitrary delegate or object path.

The default limits are 64 nodes, child depth 2, three ancestors and 32 children
per node. Accepted limits cannot exceed 128 nodes, depth 8, eight ancestors or 128
children per node. Each report also has fixed budgets of 64 type records, 16
inheritance levels, 256 methods, 256 properties and 48 KiB copied text. Strings are
bounded to 512 bytes or 512 UTF-16 units. Limit and cycle diagnostics make a
report partial. Invalid counts, text, changed identities or replaced edges
fail the inspection. Final re-resolution checks every recorded relationship
and every retained node's identity and child count.

`inspection.report()` returns only owned Rust facts. `inspection.check(guard)`
revalidates retained attachment witnesses without releasing them. It refuses
reentry. A report is an observation within a GUI transaction, not a token that
authorizes a later mutation. A later graph editor must re-resolve its own typed
attachment path under its own document and model guards. Reflection tables
with mutable class flags can change at runtime; method names do not authorize
unchecked dispatch.

The type inventory copies public `ITypeInfo` method signatures and property
declarations. It does not call those methods, enumerate dynamic instance
properties or prove that a declared method succeeds in the host. A complete
report covers its configured node routes and metadata tables. It does not
establish complete graph topology or every delegated application object.

## Verification

```sh
cargo +1.88.0 fmt --manifest-path native/node-api/Cargo.toml -- --check
cargo +1.88.0 test --manifest-path native/node-api/Cargo.toml
cargo +1.88.0 clippy --manifest-path native/node-api/Cargo.toml --all-targets -- -D warnings
sh native/node-api/checks/check_sdk.sh /path/to/ccl-framework-5.1.2
```

Tests use owned native mocks and compile-fail thread checks. The optional SDK
check validates exact commit/header hashes, arm64 virtual-call offsets and
wire layouts. It does not link the SDK or install or launch the DAW. Actual
Studio Pro 8.1.1 behavior requires separate host evidence.

Interface declarations and compiler checks were authored independently using
official CCL tag ccl-5.1.2, commit
ad5876b1a7b64aa247f4378c86a7862e7c284ba3. No CCL implementation, header or
proprietary resource is included in this crate.
