use super::{
    ffi::{self, Owned, StringRepr},
    text::NativeString,
    value::{self, Property, Variant},
    *,
};
use std::{
    ffi::c_void,
    ptr::NonNull,
    thread::{self, ThreadId},
};

#[path = "details.rs"]
mod details;
#[path = "formatting.rs"]
mod formatting;
#[path = "owner.rs"]
mod owner;
pub use owner::ControllerOwner;
#[path = "numeric_edit.rs"]
mod numeric_edit;
pub use numeric_edit::{
    IntegerEditProfile, NumericEditAttempt, NumericEditLease, NumericEditPhase,
};
#[path = "pulse.rs"]
mod pulse;
pub use pulse::{PulseError, PulseOutcome, PulseReceipt, PulseState};

/// One caller-resolved IParameter reference, owned and confined to its thread.
///
/// ```compile_fail
/// use fender_native_parameter_api::RawParameter;
/// fn send_to_worker(parameter: RawParameter) {
///     std::thread::spawn(move || drop(parameter));
/// }
/// ```
///
/// ```compile_fail
/// use fender_native_parameter_api::RawParameter;
/// fn require_sync<T: Sync>() {}
/// require_sync::<RawParameter>();
/// ```
pub struct RawParameter {
    parameter: Owned,
    text: TextApi,
    thread: ThreadId,
}
enum Expected {
    Number(Number),
    Normalized(f32),
    Text(String),
}
struct PreparedText {
    text: NativeString,
    expected: Expected,
    formatter: Option<Owned>,
    preview_parser: DisplayParser,
}

#[derive(PartialEq, Eq)]
struct ListSnapshot {
    name: String,
    tag: i32,
    count: u32,
}

impl RawParameter {
    /// Takes one owned native reference without resolving a target.
    ///
    /// # Safety
    /// `parameter` must be a live CCL 5.1.2 macOS 64-bit IParameter interface,
    /// with one reference transferred to this wrapper. The caller must have
    /// validated the native runtime ABI and current GUI thread. Text APIs must
    /// belong to that runtime and outlive this wrapper. Keep it on this thread.
    pub unsafe fn from_owned(parameter: NonNull<c_void>, text: TextApi) -> Self {
        Self {
            parameter: unsafe { Owned::take(parameter) },
            text,
            thread: thread::current().id(),
        }
    }
    pub fn as_borrowed_raw(&self) -> NonNull<c_void> {
        NonNull::new(self.parameter.raw()).expect("owned pointer is nonnull")
    }
    /// Reads one fixed String parameter's display without property reflection,
    /// formatting inputs, edits or updates. The caller pins and guards its
    /// controller/document route through use and eventual parameter release.
    pub fn string_display<G: FnMut() -> Result<(), String>>(
        &self,
        expected_name: &str,
        guard: &mut G,
    ) -> Result<String, Error> {
        self.check_thread()?;
        guard().map_err(Error::Guard)?;
        let mut output = None;
        let mut signature = None;
        let result = (|| {
            let before = self.signature();
            guard().map_err(Error::Guard)?;
            let before = before?;
            if before.0 != expected_name {
                return Err(Error::MetadataChanged);
            }
            if before.2 != Kind::String {
                return Err(Error::UnsupportedKind);
            }
            signature = Some(before);
            output = Some(NativeString::empty(self.text)?);
            guard().map_err(Error::Guard)?;
            let get: unsafe extern "C" fn(*mut c_void, *mut StringRepr) =
                unsafe { ffi::method(self.parameter.raw(), 44) };
            unsafe {
                get(
                    self.parameter.raw(),
                    output.as_mut().expect("OUT string").as_mut_ptr(),
                )
            };
            guard().map_err(Error::Guard)?;
            let copied = output.as_ref().expect("OUT string").copy();
            // copy() releases its temporary IString pin before this guard.
            guard().map_err(Error::Guard)?;
            copied
        })();
        let before_release = guard().map_err(Error::Guard);
        drop(output);
        let after_release = guard().map_err(Error::Guard);
        before_release.and(after_release)?;
        if result.is_ok() {
            // OUT String destruction can reenter without changing the route.
            // Refresh this fixed parameter's signature after that cleanup too.
            let after = self.signature();
            guard().map_err(Error::Guard)?;
            if Some(after?) != signature {
                return Err(Error::MetadataChanged);
            }
        }
        result
    }
    /// Read List labels without selecting an index. Each native label call uses
    /// IParameter.getString(String OUT, Variant const reference IN), not a
    /// nontrivial Variant return or IListParameter's selected values.
    ///
    /// A failure discards the whole page. The caller guards its retained
    /// document/target context; item contents may still change between calls.
    pub fn list_choices<G: FnMut() -> Result<(), String>>(
        &self,
        offset: u32,
        limit: u32,
        guard: &mut G,
    ) -> Result<ListChoices, Error> {
        self.check_thread()?;
        if !(1..=MAX_CHOICES_PAGE).contains(&limit) {
            return Err(Error::InvalidData("choice limit must be between 1 and 128"));
        }
        guard().map_err(Error::Guard)?;
        let before = self.list_snapshot()?;
        guard().map_err(Error::Guard)?;
        if offset > before.count {
            return Err(Error::OutOfRange);
        }
        let end = offset.saturating_add(limit).min(before.count);
        let mut choices = Vec::with_capacity((end - offset) as usize);
        for index in offset..end {
            guard().map_err(Error::Guard)?;
            let mut output = NativeString::empty(self.text)?;
            let value = Variant::number(Number::Integer(i64::from(index)), 1)?;
            let copied = (|| {
                // Empty-string construction may itself reenter the host.
                guard().map_err(Error::Guard)?;
                let current = self.list_snapshot()?;
                guard().map_err(Error::Guard)?;
                if current != before {
                    return Err(Error::MetadataChanged);
                }
                let get: unsafe extern "C" fn(*mut c_void, *mut StringRepr, *const Variant) =
                    unsafe { ffi::method(self.parameter.raw(), 43) };
                unsafe { get(self.parameter.raw(), output.as_mut_ptr(), &value) };
                guard().map_err(Error::Guard)?;
                output.copy()
            })();
            // Both the copy's own pin and the current OUT String owner must
            // finish releasing before a label/page can be accepted.
            drop(output);
            drop(value);
            guard().map_err(Error::Guard)?;
            choices.push(ListChoice {
                index,
                label: copied?,
            });
        }
        let after = self.list_snapshot()?;
        guard().map_err(Error::Guard)?;
        if before != after {
            return Err(Error::MetadataChanged);
        }
        Ok(ListChoices {
            count: before.count,
            offset,
            next: (end < before.count).then_some(end),
            choices,
        })
    }
    fn list_snapshot(&self) -> Result<ListSnapshot, Error> {
        let (name, tag, kind) = self.signature()?;
        if kind != Kind::List {
            return Err(Error::UnsupportedKind);
        }
        let object = self.parameter.query(&ffi::OBJECT)?;
        let min = value::read(&object, self.text, Property::Min)?;
        let max = value::read(&object, self.text, Property::Max)?;
        let count = match (
            min.as_ref().map(|v| &v.value),
            max.as_ref().map(|v| &v.value),
        ) {
            (Some(Value::Integer(0)), Some(Value::Integer(-1))) => 0,
            (Some(Value::Integer(0)), Some(Value::Integer(max))) if *max >= 0 => {
                if *max >= i64::from(MAX_LIST_CHOICES) {
                    return Err(Error::InvalidData("choice count exceeds 1000000"));
                }
                *max as u32 + 1
            }
            _ => {
                return Err(Error::InvalidData(
                    "List choices require integer bounds 0..max",
                ));
            }
        };
        Ok(ListSnapshot { name, tag, count })
    }
    fn check_thread(&self) -> Result<(), Error> {
        if thread::current().id() == self.thread {
            Ok(())
        } else {
            Err(Error::WrongThread)
        }
    }
    fn signature(&self) -> Result<(String, i32, Kind), Error> {
        let raw = self.parameter.raw();
        let kind: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { ffi::method(raw, 3) };
        let tag: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { ffi::method(raw, 13) };
        Ok((
            ffi::name(raw)?,
            unsafe { tag(raw) },
            Kind::from_raw(unsafe { kind(raw) }),
        ))
    }
    fn flags(&self) -> (bool, bool) {
        let raw = self.parameter.raw();
        let enabled: unsafe extern "C" fn(*mut c_void) -> u8 = unsafe { ffi::method(raw, 6) };
        let state: unsafe extern "C" fn(*mut c_void, i32) -> u8 = unsafe { ffi::method(raw, 8) };
        (
            unsafe { enabled(raw) } != 0,
            unsafe { state(raw, 0x1000) } != 0,
        )
    }
    fn formatter(&self) -> Result<Option<Owned>, Error> {
        let get: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
            unsafe { ffi::method(self.parameter.raw(), 41) };
        let pointer = unsafe { get(self.parameter.raw()) };
        if pointer.is_null() {
            Ok(None)
        } else {
            unsafe { Owned::hold(pointer) }.map(Some)
        }
    }
    fn parser(&self, kind: Kind) -> Result<DisplayParser, Error> {
        if kind == Kind::String {
            return Ok(DisplayParser::LiteralString);
        }
        // Label lists have distinct index/label semantics; keep this first
        // generic display operation limited to scalar numeric formatters.
        if !kind.numeric() || kind == Kind::List {
            return Ok(DisplayParser::Unsupported);
        }
        let Some(formatter) = self.formatter()? else {
            return Ok(DisplayParser::PlainNumeric);
        };
        let flags: unsafe extern "C" fn(*mut c_void) -> i32 =
            unsafe { ffi::method(formatter.raw(), 3) };
        Ok(DisplayParser::NativeFormatter {
            flags: unsafe { flags(formatter.raw()) },
        })
    }
    pub fn describe(&self) -> Result<Description, Error> {
        self.check_thread()?;
        let (name, tag, kind) = self.signature()?;
        let (enabled, read_only) = self.flags();
        let object = self.parameter.query(&ffi::OBJECT)?;
        let raw = value::read(&object, self.text, Property::Value)?;
        let min = value::read(&object, self.text, Property::Min)?;
        let max = value::read(&object, self.text, Property::Max)?;
        let default_value = value::read(&object, self.text, Property::Default)?;
        let normalized = if kind.numeric() {
            let get: unsafe extern "C" fn(*mut c_void) -> f32 =
                unsafe { ffi::method(self.parameter.raw(), 35) };
            let n = unsafe { get(self.parameter.raw()) };
            if n.is_finite() {
                Normalized::Finite(n)
            } else {
                Normalized::NonFinite { bits: n.to_bits() }
            }
        } else {
            Normalized::NotApplicable
        };
        let mut display = NativeString::empty(self.text)?;
        let get: unsafe extern "C" fn(*mut c_void, *mut StringRepr) =
            unsafe { ffi::method(self.parameter.raw(), 44) };
        unsafe { get(self.parameter.raw(), display.as_mut_ptr()) };
        let display = display.copy()?;
        let display_parser = self.parser(kind)?;
        let mut description = Description {
            name,
            tag,
            kind,
            enabled,
            read_only,
            raw,
            min,
            max,
            default_value,
            normalized,
            display,
            display_parser,
            support: Support {
                raw_number: false,
                normalized: false,
                display_text: false,
            },
        };
        let numeric = numeric_metadata(&description).is_ok();
        let active = enabled && !read_only;
        description.support = Support {
            raw_number: active && kind.numeric() && numeric,
            normalized: active
                && kind.numeric()
                && numeric
                && matches!(normalized, Normalized::Finite(_)),
            display_text: active
                && match display_parser {
                    DisplayParser::LiteralString => matches!(
                        description.raw.as_ref().map(|v| &v.value),
                        Some(Value::Text(_))
                    ),
                    DisplayParser::PlainNumeric => numeric,
                    DisplayParser::NativeFormatter { flags } => numeric && flags & 2 == 0,
                    DisplayParser::Unsupported => false,
                },
        };
        Ok(description)
    }
    fn begin<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<Description, MutationError> {
        self.check_thread()
            .map_err(|e| failure(e, None, None, false))?;
        guard().map_err(|e| failure(Error::Guard(e), None, None, false))?;
        self.describe().map_err(|e| failure(e, None, None, false))
    }
    fn refreshed(&self, previous: &Description) -> Result<Description, Error> {
        let fresh = self.describe()?;
        if fresh.name != previous.name || fresh.tag != previous.tag || fresh.kind != previous.kind {
            return Err(Error::MetadataChanged);
        }
        Ok(fresh)
    }
    fn final_guard<G: FnMut() -> Result<(), String>>(
        &self,
        before: &Description,
        guard: &mut G,
        preview: Option<(&Option<Owned>, DisplayParser)>,
    ) -> Result<(), Error> {
        let (name, tag, kind) = self.signature()?;
        if name != before.name || tag != before.tag || kind != before.kind {
            return Err(Error::MetadataChanged);
        }
        let (enabled, read_only) = self.flags();
        if !enabled {
            return Err(Error::Disabled);
        }
        if read_only {
            return Err(Error::ReadOnly);
        }
        if let Some((formatter, parser)) = preview {
            // Recheck after the other native guard reads. The retained preview
            // formatter keeps this borrowed interface alive without another
            // temporary retain/release after the final flags observation.
            let get: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
                unsafe { ffi::method(self.parameter.raw(), 41) };
            if unsafe { get(self.parameter.raw()) }
                != formatter.as_ref().map_or(std::ptr::null_mut(), Owned::raw)
            {
                return Err(Error::MetadataChanged);
            }
            if let DisplayParser::NativeFormatter { flags } = parser {
                let f = formatter.as_ref().ok_or(Error::MetadataChanged)?;
                let get_flags: unsafe extern "C" fn(*mut c_void) -> i32 =
                    unsafe { ffi::method(f.raw(), 3) };
                if unsafe { get_flags(f.raw()) } != flags {
                    return Err(Error::MetadataChanged);
                }
            }
        }
        // Last foreign/context operation before the actual setter.
        guard().map_err(Error::Guard)
    }
    fn finish<G: FnMut() -> Result<(), String>, C: FnOnce()>(
        &self,
        request: Request,
        before: Description,
        expected: Expected,
        guard: &mut G,
        cleanup: C,
    ) -> Result<MutationReadback, MutationError> {
        // First operation after the setter; don't read an invalidated target.
        guard().map_err(|e| failure(Error::Guard(e), Some(before.clone()), None, true))?;
        // Release all native input/formatter temporaries before observing a
        // successful result. Release itself can reenter termination callbacks.
        cleanup();
        guard().map_err(|e| failure(Error::Guard(e), Some(before.clone()), None, true))?;
        let after = self
            .describe()
            .map_err(|e| failure(e, Some(before.clone()), None, true))?;
        guard().map_err(|e| {
            failure(
                Error::Guard(e),
                Some(before.clone()),
                Some(after.clone()),
                true,
            )
        })?;
        if before.name != after.name || before.tag != after.tag || before.kind != after.kind {
            return Err(failure(
                Error::MetadataChanged,
                Some(before),
                Some(after),
                true,
            ));
        }
        if after.kind.numeric() {
            let (current, min, max) = numeric_metadata(&after)
                .map_err(|e| failure(e, Some(before.clone()), Some(after.clone()), true))?;
            // The List -1 exception allows an initial selection attempt only.
            // A verified write must leave a genuinely in-range raw index;
            // a single-entry List can report normalized 0 while still unset.
            value::in_range(current, min, max)
                .map_err(|e| failure(e, Some(before.clone()), Some(after.clone()), true))?;
        }
        let verified = match expected {
            Expected::Number(n) => value::exact(n, after.raw.as_ref()),
            Expected::Normalized(n) => {
                matches!(after.normalized,Normalized::Finite(v) if (v-n).abs()<=1e-6)
            }
            Expected::Text(text) => {
                matches!(after.raw.as_ref().map(|v|&v.value),Some(Value::Text(actual)) if actual==&text)
            }
        };
        Ok(MutationReadback {
            request,
            before,
            after,
            verified,
        })
    }
    pub fn set_raw_number<G: FnMut() -> Result<(), String>>(
        &self,
        number: Number,
        guard: &mut G,
    ) -> Result<MutationReadback, MutationError> {
        let initial = self.begin(guard)?;
        let before = self
            .refreshed(&initial)
            .map_err(|e| failure(e, Some(initial), None, false))?;
        let native = validate_number(&before, number)
            .map_err(|e| failure(e, Some(before.clone()), None, false))?;
        let tag = before
            .raw
            .as_ref()
            .ok_or_else(|| {
                failure(
                    Error::MissingValue("value"),
                    Some(before.clone()),
                    None,
                    false,
                )
            })?
            .variant_tag;
        let input = Variant::number(native, tag)
            .map_err(|e| failure(e, Some(before.clone()), None, false))?;
        self.final_guard(&before, guard, None)
            .map_err(|e| failure(e, Some(before.clone()), None, false))?;
        let set: unsafe extern "C" fn(*mut c_void, *const Variant, u8) =
            unsafe { ffi::method(self.parameter.raw(), 21) };
        unsafe { set(self.parameter.raw(), &input, 1) };
        self.finish(
            Request::RawNumber(number),
            before,
            Expected::Number(native),
            guard,
            || {},
        )
    }
    pub fn set_normalized<G: FnMut() -> Result<(), String>>(
        &self,
        normalized: f32,
        guard: &mut G,
    ) -> Result<MutationReadback, MutationError> {
        let initial = self.begin(guard)?;
        let before = self
            .refreshed(&initial)
            .map_err(|e| failure(e, Some(initial), None, false))?;
        validate_active(&before)
            .and_then(|_| numeric_metadata(&before).map(|_| ()))
            .and_then(|_| {
                if !before.support.normalized || !normalized.is_finite() {
                    Err(Error::NonFinite)
                } else if !(0.0..=1.0).contains(&normalized) {
                    Err(Error::OutOfRange)
                } else {
                    Ok(())
                }
            })
            .map_err(|e| failure(e, Some(before.clone()), None, false))?;
        self.final_guard(&before, guard, None)
            .map_err(|e| failure(e, Some(before.clone()), None, false))?;
        let set: unsafe extern "C" fn(*mut c_void, f32, u8) =
            unsafe { ffi::method(self.parameter.raw(), 36) };
        unsafe { set(self.parameter.raw(), normalized, 1) };
        self.finish(
            Request::Normalized(normalized),
            before,
            Expected::Normalized(normalized),
            guard,
            || {},
        )
    }
    pub fn set_display_text<G: FnMut() -> Result<(), String>>(
        &self,
        text: &str,
        guard: &mut G,
    ) -> Result<MutationReadback, MutationError> {
        let initial = self.begin(guard)?;
        let prepared = self
            .prepare_text(&initial, text)
            .map_err(|e| failure(e, Some(initial.clone()), None, false))?;
        let original_parser = initial.display_parser;
        let before = self
            .refreshed(&initial)
            .map_err(|e| failure(e, Some(initial), None, false))?;
        validate_active(&before).map_err(|e| failure(e, Some(before.clone()), None, false))?;
        if before.display_parser != original_parser
            || before.display_parser != prepared.preview_parser
            || !before.support.display_text
        {
            return Err(failure(Error::MetadataChanged, Some(before), None, false));
        }
        let expected = match prepared.expected {
            Expected::Number(n) => Expected::Number(
                validate_number(&before, n)
                    .map_err(|e| failure(e, Some(before.clone()), None, false))?,
            ),
            Expected::Normalized(n) => {
                numeric_metadata(&before)
                    .map_err(|e| failure(e, Some(before.clone()), None, false))?;
                if !before.support.normalized || !(0.0..=1.0).contains(&n) {
                    return Err(failure(Error::OutOfRange, Some(before), None, false));
                }
                Expected::Normalized(n)
            }
            Expected::Text(v) => {
                if before.kind != Kind::String {
                    return Err(failure(Error::TypeMismatch, Some(before), None, false));
                }
                Expected::Text(v)
            }
        };
        let current_formatter = self
            .formatter()
            .map_err(|e| failure(e, Some(before.clone()), None, false))?;
        if current_formatter.as_ref().map(Owned::raw) != prepared.formatter.as_ref().map(Owned::raw)
        {
            return Err(failure(Error::MetadataChanged, Some(before), None, false));
        }
        self.final_guard(
            &before,
            guard,
            Some((&prepared.formatter, prepared.preview_parser)),
        )
        .map_err(|e| failure(e, Some(before.clone()), None, false))?;
        let set: unsafe extern "C" fn(*mut c_void, *const StringRepr, u8) =
            unsafe { ffi::method(self.parameter.raw(), 45) };
        unsafe { set(self.parameter.raw(), prepared.text.as_ptr(), 1) };
        self.finish(
            Request::DisplayText(text.into()),
            before,
            expected,
            guard,
            move || {
                drop(current_formatter);
                drop(prepared.formatter);
                drop(prepared.text);
            },
        )
    }
    fn prepare_text(&self, before: &Description, input: &str) -> Result<PreparedText, Error> {
        validate_active(before)?;
        if !before.support.display_text {
            return Err(Error::UnsupportedDisplayParser);
        }
        let text = NativeString::from_text(self.text, input)?;
        let formatter = self.formatter()?;
        if before.kind == Kind::String {
            return Ok(PreparedText {
                text,
                expected: Expected::Text(input.into()),
                formatter,
                preview_parser: DisplayParser::LiteralString,
            });
        }
        numeric_metadata(before)?;
        let (expected, preview_parser) = if let Some(f) = formatter.as_ref() {
            let flags: unsafe extern "C" fn(*mut c_void) -> i32 =
                unsafe { ffi::method(f.raw(), 3) };
            let flags = unsafe { flags(f.raw()) };
            if flags & 2 != 0 {
                return Err(Error::UnsupportedDisplayParser);
            }
            let scan: unsafe extern "C" fn(*mut c_void, *mut Variant, *const StringRepr) -> u8 =
                unsafe { ffi::method(f.raw(), 5) };
            let mut output = Variant::empty();
            if unsafe { scan(f.raw(), &mut output, text.as_ptr()) } == 0 {
                return Err(Error::InvalidText);
            }
            let parsed = output.decode()?;
            let parsed = value::number(Some(&parsed)).ok_or(Error::TypeMismatch)?;
            let expected = if flags & 1 != 0 {
                let n = match parsed {
                    Number::Integer(n) => n as f64,
                    Number::Float(n) => n,
                };
                if !n.is_finite() {
                    return Err(Error::NonFinite);
                }
                if !(0.0..=1.0).contains(&n) {
                    return Err(Error::OutOfRange);
                }
                Expected::Normalized(n as f32)
            } else {
                Expected::Number(validate_number(before, parsed)?)
            };
            (expected, DisplayParser::NativeFormatter { flags })
        } else {
            let parsed = match value::number(before.raw.as_ref()).ok_or(Error::TypeMismatch)? {
                Number::Integer(_) => {
                    Number::Integer(input.trim().parse().map_err(|_| Error::InvalidText)?)
                }
                Number::Float(_) => {
                    Number::Float(input.trim().parse().map_err(|_| Error::InvalidText)?)
                }
            };
            (
                Expected::Number(validate_number(before, parsed)?),
                DisplayParser::PlainNumeric,
            )
        };
        Ok(PreparedText {
            text,
            expected,
            formatter,
            preview_parser,
        })
    }
}
fn failure(
    error: Error,
    before: Option<Description>,
    after: Option<Description>,
    dispatched: bool,
) -> MutationError {
    MutationError {
        outcome: if dispatched {
            Outcome::Unknown
        } else {
            Outcome::NotApplied
        },
        error,
        before: before.map(Box::new),
        after: after.map(Box::new),
    }
}
fn validate_active(before: &Description) -> Result<(), Error> {
    if !before.enabled {
        Err(Error::Disabled)
    } else if before.read_only {
        Err(Error::ReadOnly)
    } else if !before.kind.numeric() && before.kind != Kind::String {
        Err(Error::UnsupportedKind)
    } else {
        Ok(())
    }
}
fn numeric_metadata(before: &Description) -> Result<(Number, Number, Number), Error> {
    if !before.kind.numeric() {
        return Err(Error::UnsupportedKind);
    }
    let current =
        value::number(before.raw.as_ref()).ok_or(Error::MissingValue("finite numeric value"))?;
    let min =
        value::number(before.min.as_ref()).ok_or(Error::MissingValue("finite numeric min"))?;
    let max =
        value::number(before.max.as_ref()).ok_or(Error::MissingValue("finite numeric max"))?;
    // Dynamic List entries can arrive after the native current index became
    // -1. Permit selecting a valid index, without broadening any native range
    // or allowing -1 to be written back outside that range.
    let unselected_list = matches!(
        (before.kind, current, min, max),
        (Kind::List, Number::Integer(-1), Number::Integer(0), Number::Integer(max)) if max >= 0
    );
    if !unselected_list {
        value::in_range(current, min, max)?;
    }
    Ok((current, min, max))
}
fn validate_number(before: &Description, requested: Number) -> Result<Number, Error> {
    validate_active(before)?;
    let (current, min, max) = numeric_metadata(before)?;
    let requested = value::coerce(requested, current)?;
    if before.kind == Kind::Toggle
        && !matches!(requested, Number::Integer(0 | 1) | Number::Float(0.0 | 1.0))
    {
        return Err(Error::OutOfRange);
    }
    value::in_range(requested, min, max)?;
    Ok(requested)
}
