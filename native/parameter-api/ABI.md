# API facts used by the independent Rust adapter

Primary declaration source is official [CCL5.1.2](https://github.com/cclsoftware/ccl-framework/tree/ad5876b1a7b64aa247f4378c86a7862e7c284ba3).
The source tree is separate; no SDK implementation/header is distributed here.
`checks/abi_calls.cpp` compiles calls against that source and asserts layouts.
`checks/check_abi.py` checks the resulting arm64 virtual dispatch offsets.

| Interface | Method | Slot | Rust boundary |
| --- | --- | ---: | --- |
| IUnknown | queryInterface | 0 | i32 result, UID const reference, void** output |
| IUnknown | retain / release | 1 / 2 | u32 result |
| IParameter | getType / getName | 3 / 4 | i32 / const CString reference |
| IParameter | isEnabled / getState | 6 / 8 | u8 result; state takes i32 mask |
| IParameter | getTag | 13 | i32 result |
| IParameter | getController | 14 | borrowed IUnknown pointer, retained before context reentry |
| IParameter | beginEdit / endEdit | 16 / 17 | void result, no argument beyond the retained receiver |
| IParameter | getOriginal | 18 | borrowed IParameter pointer, retained before any context callback |
| IParameter | setValue | 21 | const Variant reference input; u8 update |
| IParameter | getPrecision | 30 | i32 result, reported without deriving a choice count |
| IParameter | getNormalized / setNormalized | 35 / 36 | f32 / f32 input plus u8 update |
| IParameter | getFormatter | 41 | borrowed IFormatter pointer, retained before use |
| IParameter | getString | 43 | String OUT and const numeric Variant reference IN; explicit raw-value formatting and indexed List labels |
| IParameter | toString / fromString | 44 / 45 | String OUT / const String reference IN plus u8 update |
| IObject | getProperty | 4 | Variant OUT, const CString reference ID, u8 result |
| IString | getLength / copyTo | 4 / 8 | i32 / UTF-16 OUT pointer, i32 count, i32 status |
| IString | cloneString | 18 | new owned IString pointer |
| IString | getPrivateData | 21 | pointer used in PlainString debug member |
| IString | assignChars | 24 | const UTF-16 pointer, i32 count, i32 status |
| IFormatter | getFlags / scanString | 3 / 5 | i32 / Variant OUT, const String reference IN, u8 result |

Declaration references in this exact commit:

- [IParameter](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/gui/iparameter.h#L130): parameter declarations and `kReadOnly` flag at line102.
- [Parameter controller](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/gui/iparameter.h#L167): getController returns the controller associated with this parameter. Compiler output checks slot14 at byte offset112. The adjacent performUpdate method is not used by owner observation.
- [Parameter state flags](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/gui/iparameter.h#L97): `kStorable=0x80`, `kStoreListValue=0x100`, `kMutable=0x800`, `kReadOnly=0x1000`, and `kCanUndo=0x10000`. Compiler assertions check each mask.
- [IUnknown identity](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/base/iunknown.cpp#L23): canonical query IID `00000000-0000-0000-c000-000000000046`.
- [IListParameter](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/gui/iparameter.h#L550): query IID `143641b5-92a0-4a2e-b70c-84c57e8fbdb3`. The details helper only queries and releases this interface.
- [IObject](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/base/iobject.h#L172): `getProperty` output reference.
- [IString](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/text/istring.h#L102): length/copy/clone/private/assign methods.
- [IFormatter](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/base/iformatter.h#L58): normalized/stateful flags and scan output reference.
- [Variant](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/base/variant.h): scalar tags, shared owner flag and payload ABI.
- [String layout](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/text/cclstring.h) and [CString layout](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/public/text/cstring.h): PlainString/PlainCString representation and reference aliases.

The C++ Variant is nontrivially destructible and nontrivially copyable, size16,
align8. Its by-value return is an indirect x8 result on this target. The adapter
therefore deliberately excludes `IParameter::getValue/getMin/getMax/getDefault`
and other C++ Variant-return methods. It uses a non-Copy 16-byte OUT carrier
with signed16 tag at0 and scalar/pointer payload at8. A shared String/Object
carrier releases its **current** payload owner once on Drop. A String Variant's
payload is a direct IString interface pointer, not a PlainString address.

Fixed property semantics and List raw-index behavior were checked in the same
official `ccl/app/params.cpp` source as interoperability facts. The adapter does
not copy those implementations or depend on their classes' object layouts.
It treats absence/failure/type mismatch as explicit errors or unsupported data.

The finite button pulse follows the parameter-call sequence in official
[Button::push](https://github.com/cclsoftware/ccl-framework/blob/ad5876b1a7b64aa247f4378c86a7862e7c284ba3/ccl/gui/controls/button.cpp#L862).
It supplies max with update=true and then min with update=false through the
existing compiler-checked `setValue` reference-input slot21. Bounds come from
guarded IObject OUT properties, so the adapter adds no C++ Variant-by-value
call. The independent implementation pins and checks the parameter/controller
association and stops after an unknown upper dispatch. It implements no view
signal and makes no inference that a Toggle or signal flag is an action.

The separate consuming numeric edit operation brackets one raw numeric update
with public `beginEdit` and `endEdit`. Compiler probes check byte offsets128
and136 and both void/no-argument declarations. Calls use the same selected
receiver and preserve the existing direct setter path. The operation retains
and rechecks the finite original/controller binding through publication.
After begin dispatch any failure is Unknown. Attachment loss prevents set/end;
a metadata-only failure may end once on a freshly proved attachment. Drop
only releases references. Neither a closed phase nor live readback proves
serialization, undo behavior or audio processing.
The final writable witness uses a retained IObject view and pre-resolved numeric
property IDs after the last callback-bearing route guard. It rechecks the raw
variant, range, signature and active flags using scalar OUT values. Unexpected
owned output is released on the refusal path before any safe-close decision;
successful witness cleanup invokes no native release.
Once closed, the retained witness also requires the exact observed readback
value to remain unchanged through publication. A stable readback mismatch may
still report verified=false; a later value change invalidates the receipt.

The public `IParameter::getString` declaration formats a supplied value through
an OUT String and const Variant reference. The List implementation's
index-to-label behavior was checked as an interoperability fact in the same
official source. A List may dynamically grow its maximum from -1 while its
current raw integer remains -1. The adapter independently permits only that
specific unselected integer state to initialize an index already inside its
valid integer 0..max range. It neither copies the CCL List implementation nor
uses its nontrivial `getValueAt`/selected-value return ABI.

The read-only original-chain helper calls `getOriginal` at slot18 and
`getPrecision` at slot30. Compiler output resolves these to byte offsets144 and
240. It pins canonical IUnknown identities to deduplicate views and cycles.
Neither precision, state flags, IListParameter capability, nor Float bounds
establish a choice count. The helper adds no by-value Variant call or setter.
The newer-SDK regression compiles with `FENDER_PARAMETER_CHECK_OFFSETS_ONLY`
to compare dispatch independently of the version-specific state masks. The
ordinary 5.1.2 check asserts every mask and retains the exact header pins.

These source facts and local compiler probes establish the reviewed **source
ABI**, not actual compatibility with an arbitrary installed runtime. The owning
add-on must validate the exact Fender8.1.1 main and CCL runtime before use.
