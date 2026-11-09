# One executor owns the DAW

The MCP client reads the active document, then sends typed edits with that
document token. The executor returns what it applied, what it verified, and
anything whose outcome remains unknown.

One native owner thread owns the CoreMIDI source, mailbox and the complete
write-and-readback sequence. A profile lock rejects a second controller process.
MCP handlers submit bounded requests and never write mailbox files directly.
Rust domain operations are finite. A raw arbitrary script endpoint is not part
of the API.

Bridge protocol 2 carries the request ID, expected bridge session and document
token. The bridge checks the active object before dispatch and after the
operation. A document token tracks observed active-document object changes.
A response must match the request and bridge session.

An EditTask consumes a short-lived request and lease before applying work and
independently checks session, path and title. This task boundary cannot prove
the document lifetime if a same-path reopen occurs during synchronous dispatch;
the outer bridge detects the changed object afterward and reports an unknown
outcome. See [validation limits](VALIDATION.md).

A timeout after dispatch can mean the host edited the session but the reply was
lost. The executor reports an unknown outcome and does not repeat non-idempotent
edits. Repeated MIDI wakeups for one request ID are deduplicated in the bridge.
This is not crash-safe exactly-once execution.

The optional Rust CCL extension runs inside the DAW on its GUI thread. The
bridge passes an already resolved native Device object through the scripting
engine's own marshaller, with a finite JSON request. The native service exposes
capabilities and a bounded request method; it has no evaluator, arbitrary method
dispatcher, native-address input, shell or second mailbox executor. The Rust
server's existing profile lock and serialized operation path remain in charge.

The extension attaches its own module to the public scripting manager and
registers a fixed IObject service. Exact app/cclgui image identities are checked
before CCL virtual calls. The private DSP count/info ABI supplies indexed names,
titles, tags and flags. The bridge resolves those names on the live Device.
Controller parameter access uses the pinned public CCL 5.1.2 ABI. No nontrivial
Variant return is called directly from Rust; values use OUT/IN references and
owned string/object carriers.

Native requests retain the document/model and Device identity, guard immediately
around dispatch, and retain at most one completed receipt's native owners through
response publication. Temporary native cleanup precedes the final borrowed
checks. A subsequent call or stop drains that bounded receipt. Own code and
bundle metadata are intentionally pinned until process exit; hot unloading is
not supported. Normal termination must still release service, factory, metadata,
module registration and receipt ownership.

External Effect needs its editor controller. A bounded resolver matches the
actual Device UID/class and canonical identity through a unique editor window's
DeviceSlot/Device, then checks the fixed controller path. It revalidates window
membership, relationships and identities, including after native temporary
cleanup. It never chooses an editor from a matching caption alone.

Parameter pages bind count, indices, source and instance. The native API caps a
request at 16 KiB, a reply at 256 KiB and a page at 128 records. A provider's
complete list does not establish every nested, selected-pad or model context.
Carry both the bridge document token and native instance UID between pages.
Generic writes exclude disabled/read-only fields, opaque objects and unsupported
types. The JSON/JavaScript boundary limits numeric integer transport to the safe
range; larger native integers retain exact decimal metadata and are not offered
as generic writable numbers.

Without the native extension, installed factory metadata remains partial
candidate discovery. Native failures do not silently become a claim of complete
metadata discovery. See the [native integration contracts](../native/addon/README.md)
and [actual per-class evidence](BUILTIN_PLUGINS.md).
