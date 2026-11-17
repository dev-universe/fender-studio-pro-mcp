use super::*;

/// Numeric observations needed by a trusted, separately registered button
/// action. No formatter, normalized value or default getter is called.
#[derive(Clone, Debug, PartialEq)]
pub struct PulseState {
    pub name: String,
    pub tag: i32,
    pub kind: Kind,
    pub enabled: bool,
    pub read_only: bool,
    pub value: NativeValue,
    pub min: NativeValue,
    pub max: NativeValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PulseOutcome {
    AppliedUnverified,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PulseReceipt {
    pub before: PulseState,
    pub after: PulseState,
    pub upper_dispatched: bool,
    pub reset_dispatched: bool,
    pub reset_verified: bool,
    /// Observing the reset value never verifies the button's separate effect.
    pub effects_verified: bool,
    pub outcome: PulseOutcome,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PulseError {
    pub error: Error,
    pub outcome: Outcome,
    pub before: Option<Box<PulseState>>,
    pub after: Option<Box<PulseState>>,
    pub upper_dispatched: bool,
    pub reset_dispatched: bool,
    pub effects_verified: bool,
}
impl std::fmt::Display for PulseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.outcome, self.error)
    }
}
impl std::error::Error for PulseError {}

#[derive(Clone, Copy)]
enum Phase {
    Prepared,
    UpperDispatched,
    ResetDispatched,
}
struct Progress {
    phase: Phase,
    before: Option<PulseState>,
    after: Option<PulseState>,
}
impl Progress {
    fn error(self, error: Error) -> PulseError {
        let upper = !matches!(self.phase, Phase::Prepared);
        PulseError {
            error,
            outcome: if upper {
                Outcome::Unknown
            } else {
                Outcome::NotApplied
            },
            before: self.before.map(Box::new),
            after: self.after.map(Box::new),
            upper_dispatched: upper,
            reset_dispatched: matches!(self.phase, Phase::ResetDispatched),
            effects_verified: false,
        }
    }
}

impl RawParameter {
    /// Dispatches the SDK button sequence max/update=true, min/update=false.
    ///
    /// A trusted integration must separately prove and register the action's
    /// class and parameter. Toggle type or signal flags alone do not establish
    /// button semantics. This is not an unrestricted client action API.
    ///
    /// The receiver and caller's document/provider pins must remain alive
    /// through receipt publication. Additional owners are released under the
    /// caller guard before returning. Once upper dispatch begins, any failure
    /// is unknown and prevents an automatic reset, retry or further write.
    pub fn pulse_range<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<PulseReceipt, PulseError> {
        let mut progress = Progress {
            phase: Phase::Prepared,
            before: None,
            after: None,
        };
        if let Err(error) = self.check_thread() {
            return Err(progress.error(error));
        }
        let mut object = None;
        let mut owner = None;
        let mut upper = None;
        let mut lower = None;
        let result = (|| {
            call(guard, || {
                let get: unsafe extern "C" fn(*mut c_void) -> i32 =
                    unsafe { ffi::method(self.parameter.raw(), 3) };
                Ok(Kind::from_raw(unsafe { get(self.parameter.raw()) }))
            })
            .and_then(|kind| {
                if kind == Kind::Toggle {
                    Ok(())
                } else {
                    Err(Error::UnsupportedKind)
                }
            })?;
            guard().map_err(Error::Guard)?;
            let queried = self.parameter.query(&ffi::OBJECT);
            let query_error = match queried {
                Ok(value) => {
                    object = Some(value);
                    None
                }
                Err(error) => Some(error),
            };
            guard().map_err(Error::Guard)?;
            if let Some(error) = query_error {
                return Err(error);
            }
            let object = object.as_ref().expect("successful IObject query");
            let before = self.pulse_state(object, guard)?;
            progress.before = Some(before.clone());
            validate(&before)?;
            owner = self.controller_owner(guard)?;
            let owner_pointer = match owner.as_ref() {
                Some(owner) => Some(unsafe { owner.as_borrowed_unknown() }?.as_ptr()),
                None => None,
            };
            upper = Some(Variant::number(
                number(&before.max)?,
                before.max.variant_tag,
            )?);
            lower = Some(Variant::number(
                number(&before.min)?,
                before.min.variant_tag,
            )?);
            self.pulse_gate(object, owner.as_ref(), owner_pointer, &before, guard)?;
            let set: unsafe extern "C" fn(*mut c_void, *const Variant, u8) =
                unsafe { ffi::method(self.parameter.raw(), 21) };
            progress.phase = Phase::UpperDispatched;
            unsafe {
                set(
                    self.parameter.raw(),
                    upper.as_ref().expect("prepared maximum"),
                    1,
                );
            }
            guard().map_err(Error::Guard)?;
            let after_upper = self.pulse_state(object, guard)?;
            progress.after = Some(after_upper.clone());
            validate(&after_upper)?;
            same_shape(&before, &after_upper)?;
            // An action may synchronously reset its own Toggle to zero. The
            // observed binary value is not evidence of the separate effect.
            self.pulse_gate(object, owner.as_ref(), owner_pointer, &before, guard)?;
            progress.phase = Phase::ResetDispatched;
            unsafe {
                set(
                    self.parameter.raw(),
                    lower.as_ref().expect("prepared minimum"),
                    0,
                );
            }
            guard().map_err(Error::Guard)?;
            let after = self.pulse_state(object, guard)?;
            progress.after = Some(after.clone());
            validate(&after)?;
            same_shape(&before, &after)?;
            self.pulse_owner(owner.as_ref(), guard)?;
            self.pulse_owner_view(owner_pointer, guard)?;
            let expected = value::coerce(number(&before.min)?, number(&before.value)?)?;
            if !value::exact(expected, Some(&after.value)) {
                return Err(Error::InvalidData(
                    "button reset did not match the lower bound",
                ));
            }
            Ok(PulseReceipt {
                before,
                after,
                upper_dispatched: true,
                reset_dispatched: true,
                reset_verified: true,
                effects_verified: false,
                outcome: PulseOutcome::AppliedUnverified,
            })
        })();
        // Native ownership cleanup is mandatory even after context loss. Every
        // release remains surrounded by guard observations, and no write follows.
        let upper_released = release(upper, guard);
        let lower_released = release(lower, guard);
        let owner_released = match owner {
            Some(owner) => owner.release(guard),
            None => Ok(()),
        };
        let object_released = release(object, guard);
        let final_guard = guard().map_err(Error::Guard);
        let cleanup = upper_released
            .and(lower_released)
            .and(owner_released)
            .and(object_released)
            .and(final_guard);
        match cleanup.and(result) {
            Ok(receipt) => Ok(receipt),
            Err(error) => Err(progress.error(error)),
        }
    }

    fn pulse_owner<G: FnMut() -> Result<(), String>>(
        &self,
        owner: Option<&ControllerOwner>,
        guard: &mut G,
    ) -> Result<(), Error> {
        if let Some(owner) = owner {
            return owner.check(guard);
        }
        match self.controller_owner(guard)? {
            None => Ok(()),
            Some(unexpected) => {
                unexpected.release(guard)?;
                Err(Error::MetadataChanged)
            }
        }
    }
    fn pulse_owner_view<G: FnMut() -> Result<(), String>>(
        &self,
        expected: Option<*mut c_void>,
        guard: &mut G,
    ) -> Result<(), Error> {
        let current = call(guard, || {
            let get: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
                unsafe { ffi::method(self.parameter.raw(), 14) };
            Ok(unsafe { get(self.parameter.raw()) })
        })?;
        if current == expected.unwrap_or(std::ptr::null_mut()) {
            Ok(())
        } else {
            Err(Error::MetadataChanged)
        }
    }
    fn pulse_gate<G: FnMut() -> Result<(), String>>(
        &self,
        object: &Owned,
        owner: Option<&ControllerOwner>,
        owner_pointer: Option<*mut c_void>,
        before: &PulseState,
        guard: &mut G,
    ) -> Result<(), Error> {
        self.pulse_owner(owner, guard)?;
        let current = self.pulse_state(object, guard)?;
        validate(&current)?;
        same_shape(before, &current)?;
        self.pulse_owner_view(owner_pointer, guard)?;
        guard().map_err(Error::Guard)
    }
    fn pulse_state<G: FnMut() -> Result<(), String>>(
        &self,
        object: &Owned,
        guard: &mut G,
    ) -> Result<PulseState, Error> {
        let parameter = self.parameter.raw();
        let kind = call(guard, || {
            let get: unsafe extern "C" fn(*mut c_void) -> i32 =
                unsafe { ffi::method(parameter, 3) };
            Ok(Kind::from_raw(unsafe { get(parameter) }))
        })?;
        if kind != Kind::Toggle {
            return Err(Error::UnsupportedKind);
        }
        let name = call(guard, || ffi::name(parameter))?;
        if name.is_empty() {
            return Err(Error::InvalidData("button parameter name is empty"));
        }
        let tag = call(guard, || {
            let get: unsafe extern "C" fn(*mut c_void) -> i32 =
                unsafe { ffi::method(parameter, 13) };
            Ok(unsafe { get(parameter) })
        })?;
        let enabled = call(guard, || {
            let get: unsafe extern "C" fn(*mut c_void) -> u8 = unsafe { ffi::method(parameter, 6) };
            Ok(unsafe { get(parameter) } != 0)
        })?;
        let read_only = call(guard, || {
            let get: unsafe extern "C" fn(*mut c_void, i32) -> u8 =
                unsafe { ffi::method(parameter, 8) };
            Ok(unsafe { get(parameter, 0x1000) } != 0)
        })?;
        let value = numeric_property(object, self.text, c"value", guard)?;
        let min = numeric_property(object, self.text, c"min", guard)?;
        let max = numeric_property(object, self.text, c"max", guard)?;
        Ok(PulseState {
            name,
            tag,
            kind,
            enabled,
            read_only,
            value,
            min,
            max,
        })
    }
}

fn call<T, G: FnMut() -> Result<(), String>>(
    guard: &mut G,
    native: impl FnOnce() -> Result<T, Error>,
) -> Result<T, Error> {
    guard().map_err(Error::Guard)?;
    let result = native();
    guard().map_err(Error::Guard)?;
    result
}
fn release<T, G: FnMut() -> Result<(), String>>(owner: T, guard: &mut G) -> Result<(), Error> {
    let before = guard().map_err(Error::Guard);
    drop(owner);
    let after = guard().map_err(Error::Guard);
    before.and(after)
}
fn numeric_property<G: FnMut() -> Result<(), String>>(
    object: &Owned,
    api: TextApi,
    name: &'static std::ffi::CStr,
    guard: &mut G,
) -> Result<NativeValue, Error> {
    let id = call(guard, || ffi::property_id(api, name))?;
    let mut output = Variant::empty();
    let get: unsafe extern "C" fn(*mut c_void, *mut Variant, *const c_void) -> u8 =
        unsafe { ffi::method(object.raw(), 4) };
    let result = (|| {
        let accepted = call(guard, || {
            Ok(unsafe { get(object.raw(), &mut output, id) } != 0)
        })?;
        if !accepted {
            return Err(Error::MissingValue("button numeric property"));
        }
        if !matches!(output.tag & 15, 1 | 2) {
            return Err(Error::TypeMismatch);
        }
        let value = output.decode()?;
        number(&value)?;
        Ok(value)
    })();
    release(output, guard).and(result)
}
fn number(value: &NativeValue) -> Result<Number, Error> {
    match value.value {
        Value::Integer(value) => Ok(Number::Integer(value)),
        Value::Float(value) if value.is_finite() => Ok(Number::Float(value)),
        Value::NonFiniteFloat { .. } => Err(Error::NonFinite),
        _ => Err(Error::TypeMismatch),
    }
}
fn binary(value: Number) -> bool {
    match value {
        Number::Integer(value) => matches!(value, 0 | 1),
        Number::Float(value) => value == 0.0 || value == 1.0,
    }
}
fn validate(state: &PulseState) -> Result<(), Error> {
    if state.kind != Kind::Toggle {
        return Err(Error::UnsupportedKind);
    }
    if !state.enabled {
        return Err(Error::Disabled);
    }
    if state.read_only {
        return Err(Error::ReadOnly);
    }
    let current = number(&state.value)?;
    let min = number(&state.min)?;
    let max = number(&state.max)?;
    if !binary(current)
        || value::compare(min, Number::Integer(0)) != std::cmp::Ordering::Equal
        || value::compare(max, Number::Integer(1)) != std::cmp::Ordering::Equal
    {
        return Err(Error::OutOfRange);
    }
    Ok(())
}
fn same_shape(before: &PulseState, after: &PulseState) -> Result<(), Error> {
    if before.name != after.name
        || before.tag != after.tag
        || before.kind != after.kind
        || before.enabled != after.enabled
        || before.read_only != after.read_only
        || before.value.variant_tag != after.value.variant_tag
        || !same_number(&before.min, &after.min)
        || !same_number(&before.max, &after.max)
    {
        return Err(Error::MetadataChanged);
    }
    Ok(())
}
fn same_number(a: &NativeValue, b: &NativeValue) -> bool {
    a.variant_tag == b.variant_tag && number(a).is_ok_and(|n| value::exact(n, Some(b)))
}

#[cfg(test)]
#[path = "pulse_tests.rs"]
mod tests;
