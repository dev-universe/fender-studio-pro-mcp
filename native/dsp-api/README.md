# Native DSP parameter enumeration

A std-only, read-only Rust library for one caller-resolved DSP target in the
validated **Fender Studio Pro 8.1.1 Build 113103 arm64** runtime. It enumerates the
native provider's full indexed parameter metadata, using an undocumented,
exact-version interface rather than a public CCL parameter-provider API.

The Rust implementation is independently authored and MIT licensed. It includes
no vendor headers, implementation, app resource, raw song or app log. Normal
Cargo build/test needs no DAW, CCL library, SDK checkout or C++ tools. There is no
loader, UI scan, resolver, setter, IPC, installation or symbol lookup.

## Integration contract

Use it from a sibling add-on crate:

```toml
[dependencies]
fender-native-dsp-api = { path = "../dsp-api" }
```

`unsafe DspParameters::from_owned_target(target, text_api)` consumes one owned
IUnknown target reference, including on error. Retain a borrowed target before
transferring it. It performs normal QueryInterface for the exact provider IID
and retains the resulting provider until Drop; the target outlives that provider
release. It never reads private fields from a host object.

Before construction, the caller must verify the GUI thread and both native
runtime image identities:

| Runtime image | Mach-O LC_UUID |
| --- | --- |
| Fender Studio Pro 8.1.1 arm64 executable | `6B547378-B3BB-322A-87C9-901493FD2BED` |
| Its CCL5.1.2 cclgui framework | `1A8C66AE-250D-3C7F-801C-6904D5082F57` |

Reject other runtimes before construction. A successful QueryInterface alone
does not establish compatibility. The caller retains document/model/target
context throughout use and Drop, serializes access, and validates that context
before and after each call. The library does not identify the active document
or detect reentrant service termination on its own. Use and destroy it on the
same validated GUI thread; creator-thread checks and `!Send`/`!Sync` are
additional protection. No native exception or Rust panic may unwind through an
FFI boundary.

Supply already resolved public `CCLGetConstantString` and
`CCLGetConstantCString` functions through `TextApi`. Each returns a borrowed
reference to a 16-byte native object; these function pointers must belong to the
validated CCL5.1.2 runtime and outlive the wrapper. Only the empty-string
constants are requested. The library neither loads a native library nor mutates
the constants.

## API and bounds

- `count() -> Result<u32, Error>` validates the native signed count in
  `0..=1_000_000`.
- `page(offset, limit) -> Result<Page, Error>` attempts between 1 and 128 indices
  per call, starting at an offset no greater than the reported count. It never
  scans another target or an entire UI tree.
- Records contain `index`, native `tag`, canonical `name`, display `title`,
  numeric `folder` and uninterpreted `raw_flags`. Names/titles are copied into
  Rust-owned strings before native output cleanup.
- `failed_indices` contains each failed index and its reason. A failed entry
  does not suppress later entries. `next` advances past all attempted indices;
  callers must retry failures explicitly if needed. `cap` is always the hard
  per-call cap of 128; `attempted` records actual work done.

`complete` is deliberately conservative: it is true only when **this single
call starts at zero, reaches the reported end and has no failed indices**. It
does not become true merely because a later page reaches the end. A paginated
consumer must check target identity and stable counts across calls, retain all
successful records, and prove aggregate index coverage itself. The library
rechecks the count after output cleanup and rejects a changed count. Equal
counts alone do not prove unchanged parameter identities between calls.

Count zero is a valid, complete empty result. Empty names/titles can be valid
native metadata; a name is not automatically a writable ID or a unique global
identity. Folder/tag/flag values retain their native numeric form. No writable
capability or meanings for undocumented flag bits are inferred. Completeness
refers to this one DSP provider, not sampler zones, program/layer controllers,
embedded plug-ins, UI-only controls or every possible native scope.

Names are bounded to 512 UTF-8 bytes and titles to 512 UTF-16 units. Invalid
UTF-8/UTF-16, unsuccessful title copy, missing termination, changed string length
and invalid String owners become index failures, never silent truncation or a
fake empty title. A count outside its bound, invalid page arguments, missing
constants or provider failure is a call error. No native method can be safely
preempted once entered; the limit bounds the number of calls, not their duration.

## Exact ABI and provenance

The private interface IID is `{D889CAEA-8356-45BB-8B45-EECD1131A94D}`. After the
normal IUnknown methods, slot3 returns a signed32 count and slot4 accepts an
initialized 128-byte record and signed32 index. Its observed status is exactly
0 or 1; the adapter reads a single low byte and accepts **only 1**. This is an
observed 8.1.1 compatibility fact, not a recovered public declaration. Other
statuses are explicit failures.

The independently established record layout is size128/alignment8. Compile-time
assertions check every field offset:

| Offset | Storage | Exposed meaning |
| ---: | --- | --- |
| 0 / 4 | u32 / u32 | tag / folder |
| 8 | PlainCString16 | canonical name |
| 24 | PlainCString16 | unexposed, meaning unknown |
| 40 | PlainString16 | display title |
| 56 / 72 | PlainString16 / PlainString16 | unexposed, meanings unknown |
| 88 | PlainCString16 | unexposed, meaning unknown |
| 104 / 108 | f32 / u32 | unexposed, meanings unknown |
| 112 | u32 | uninterpreted flags |
| 116 / 120 | u32 / u64 | unexposed, meanings unknown |

The CCL text/ownership facts come from official tag
[`ccl-5.1.2`](https://github.com/cclsoftware/ccl-framework/tree/ad5876b1a7b64aa247f4378c86a7862e7c284ba3),
commit `ad5876b1a7b64aa247f4378c86a7862e7c284ba3`:
[PlainString](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/text/cclstring.h),
[PlainCString](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/text/cstring.h),
[text constants](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/textservices.h),
[IString length/copy](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/text/istring.h),
and [IUnknown](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/base/iunknown.h).
No CCL implementation is copied, translated or linked.

Each record starts with valid borrowed empty String/CString representations and
retains one reference for each of its six owning fields. It is never all-zero
initialized. After every native return, including failure, RAII releases the six
**current** owner pointers in reverse layout order. A PlainString owns its first
word; a PlainCString owns its second word when nonnull. Raw CString text/debug
pointers are never released. There is no C++ Variant or nontrivial by-value
return in this interface.

An isolated prior 8.1.1 runtime probe copied all reported entries for Pro EQ
(199), Mojito (37) and Multi Instrument (150), with zero per-entry failures.
Those results support this exact interface/layout. The new reusable library's
own verification is mock-based; it must not be presented as a separate actual
host test or a promise of all native scope coverage.

## Verification

```sh
cargo +1.88.0 fmt --manifest-path native/dsp-api/Cargo.toml -- --check
cargo +1.88.0 test --manifest-path native/dsp-api/Cargo.toml
cargo +1.88.0 clippy --manifest-path native/dsp-api/Cargo.toml --all-targets -- -D warnings
```

The portable 64-bit mock tests cover pagination/caps, complete/incomplete
coverage, status failures, current-owner replacement/cleanup on all outcomes,
invalid constants/counts/arguments, changed counts, provider/target ownership,
strict title and name copies, and wrong-thread rejection. Compile-fail tests
verify the wrapper cannot be sent or shared between threads. No check opens a
DAW, loads its libraries, touches a profile or needs an SDK checkout.
