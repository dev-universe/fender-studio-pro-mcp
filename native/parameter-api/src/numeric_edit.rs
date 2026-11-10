use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericEditPhase {
    Prepared,
    BeginDispatched,
    ValueDispatched,
    EndDispatched,
    ClosedChecked,
}
impl NumericEditPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::BeginDispatched => "begin_dispatched",
            Self::ValueDispatched => "value_dispatched",
            Self::EndDispatched => "end_dispatched",
            Self::ClosedChecked => "closed_checked",
        }
    }
    pub fn effect_may_have_started(self) -> bool {
        self != Self::Prepared
    }
}

struct Link {
    owner: Option<ControllerOwner>,
    original: Owned,
    original_identity: Owned,
    identity: Owned,
    parameter: RawParameter,
}
struct NumericAccess {
    object: Owned,
    ids: [*const c_void; 3],
}
enum EditProfile {
    General,
    ExactToggle(&'static str),
}
impl EditProfile {
    fn validate(&self, description: &Description) -> Result<(), Error> {
        let Self::ExactToggle(name) = self else {
            return Ok(());
        };
        let boolean = |value: Option<&NativeValue>| match value {
            Some(NativeValue {
                variant_tag: 1,
                value: Value::Integer(number),
            }) if *number == 0 || *number == 1 => Some((*number == 1, 1)),
            Some(NativeValue {
                variant_tag: 2,
                value: Value::Float(number),
            }) if matches!(number.to_bits(), 0 | 0x3ff0_0000_0000_0000) => {
                Some((*number == 1.0, 2))
            }
            _ => None,
        };
        let raw = boolean(description.raw.as_ref());
        if description.name != *name
            || description.kind != Kind::Toggle
            || !description.enabled
            || description.read_only
            || !description.support.raw_number
            || raw.is_none()
            || boolean(description.min.as_ref()) != raw.map(|(_, carrier)| (false, carrier))
            || boolean(description.max.as_ref()) != raw.map(|(_, carrier)| (true, carrier))
        {
            return Err(Error::InvalidData(
                "numeric edit requires the exact writable Toggle01 profile",
            ));
        }
        Ok(())
    }
}

/// Retains the complete selected parameter binding through reply publication.
/// Its destructor only releases references, including after an unclosed edit.
pub struct NumericEditLease {
    links: Vec<Link>,
    numeric: Option<NumericAccess>,
    parameter: RawParameter,
    phase: NumericEditPhase,
    binding_complete: bool,
    witness: Option<(Description, Number)>,
    profile: EditProfile,
}
pub struct NumericEditAttempt {
    pub result: Result<MutationReadback, MutationError>,
    pub lease: NumericEditLease,
}

impl RawParameter {
    /// Performs one synchronous public begin/value/end interaction on this
    /// selected receiver. The integration guards its document and provider and
    /// retains the returned lease until after its final publication check.
    pub fn edit_raw_number<G: FnMut() -> Result<(), String>>(
        self,
        number: Number,
        guard: &mut G,
    ) -> NumericEditAttempt {
        self.edit_number(number, EditProfile::General, guard)
    }
    /// Edits one named Toggle with canonical numeric zero/one carriers and
    /// endpoints. Profile failures use the existing metadata failure path.
    pub fn edit_raw_toggle<G: FnMut() -> Result<(), String>>(
        self,
        expected_name: &'static str,
        enabled: bool,
        guard: &mut G,
    ) -> NumericEditAttempt {
        self.edit_number(
            Number::Integer(i64::from(enabled)),
            EditProfile::ExactToggle(expected_name),
            guard,
        )
    }
    fn edit_number<G: FnMut() -> Result<(), String>>(
        self,
        number: Number,
        profile: EditProfile,
        guard: &mut G,
    ) -> NumericEditAttempt {
        let mut lease = NumericEditLease {
            links: Vec::new(),
            numeric: None,
            parameter: self,
            phase: NumericEditPhase::Prepared,
            binding_complete: false,
            witness: None,
            profile,
        };
        let mut before = None;
        let mut after = None;
        let result = (|| {
            lease.parameter.check_thread()?;
            lease.capture(guard)?;
            lease.capture_numeric(guard)?;
            let initial = lease.description(guard)?;
            lease.profile.validate(&initial)?;
            let native = validate_number(&initial, number)?;
            before = Some(initial.clone());
            let tag = initial
                .raw
                .as_ref()
                .ok_or(Error::MissingValue("value"))?
                .variant_tag;
            let input = Variant::number(native, tag)?;
            lease
                .writable(&initial, number, guard)
                .map_err(GateError::error)?;
            lease.phase = NumericEditPhase::BeginDispatched;
            lease.dispatch_lifecycle(16);
            lease.check(guard)?;
            if let Err(error) = lease.writable(&initial, number, guard) {
                return Err(lease.close_after_gate_error(error, guard));
            }
            lease.phase = NumericEditPhase::ValueDispatched;
            let set: unsafe extern "C" fn(*mut c_void, *const Variant, u8) =
                unsafe { ffi::method(lease.parameter.parameter.raw(), 21) };
            unsafe { set(lease.parameter.parameter.raw(), &input, 1) };
            lease.check(guard)?;
            if let Err(error) = lease.writable(&initial, number, guard) {
                return Err(lease.close_after_gate_error(error, guard));
            }
            lease.phase = NumericEditPhase::EndDispatched;
            lease.dispatch_lifecycle(17);
            lease.check(guard)?;
            let current = lease.description(guard)?;
            after = Some(current.clone());
            same_numeric_shape(&initial, &current)?;
            validate_number(&current, number)?;
            let (value, min, max) = numeric_metadata(&current)?;
            value::in_range(value, min, max)?;
            lease.check(guard)?;
            lease.witness = Some((current.clone(), number));
            lease.phase = NumericEditPhase::ClosedChecked;
            Ok(MutationReadback {
                request: Request::RawNumber(number),
                before: initial,
                verified: value::exact(native, current.raw.as_ref()),
                after: current,
            })
        })();
        // All closure-local native owners have been released before this final
        // check. Persistent association pins remain in the returned lease.
        let result = match result {
            Ok(value) => match lease.check(guard) {
                Ok(()) => Ok(value),
                Err(error) => Err(error),
            },
            Err(error) => {
                let observed = guard().map_err(Error::Guard);
                Err(observed.err().unwrap_or(error))
            }
        }
        .map_err(|error| failure(error, before, after, lease.phase.effect_may_have_started()));
        NumericEditAttempt { result, lease }
    }
}

impl NumericEditLease {
    pub fn phase(&self) -> NumericEditPhase {
        self.phase
    }

    /// Rechecks retained associations without opening, closing or repairing an
    /// edit. Stable null declared controllers remain explicit null bindings.
    pub fn check<G: FnMut() -> Result<(), String>>(&self, guard: &mut G) -> Result<(), Error> {
        self.parameter.check_thread()?;
        guard().map_err(Error::Guard)?;
        if !self.binding_complete {
            return Err(Error::InvalidData("numeric edit binding is incomplete"));
        }
        for link in &self.links {
            check_identity(&link.parameter.parameter, &link.identity, guard)?;
            check_identity(&link.original, &link.original_identity, guard)?;
            if let Some(owner) = &link.owner {
                owner.check(guard)?;
            } else if !controller_pointer(link.parameter.parameter.raw()).is_null() {
                guard().map_err(Error::Guard)?;
                return Err(Error::MetadataChanged);
            }
            guard().map_err(Error::Guard)?;
        }
        if let Some(numeric) = &self.numeric {
            guard().map_err(Error::Guard)?;
            let observed = self.parameter.parameter.query(&ffi::OBJECT);
            let matched = observed
                .as_ref()
                .is_ok_and(|view| view.raw() == numeric.object.raw());
            let error = observed.as_ref().err().cloned();
            let before_release = guard().map_err(Error::Guard);
            drop(observed);
            let after_release = guard().map_err(Error::Guard);
            before_release.and(after_release)?;
            if !matched {
                return Err(error.unwrap_or(Error::MetadataChanged));
            }
        }
        self.pinned_edges(guard)?;
        if let Some((before, number)) = &self.witness {
            let observed = self.scalar_witness(before, *number)?;
            let expected = value::number(before.raw.as_ref())
                .ok_or(Error::MissingValue("closed numeric readback"))?;
            if !value::exact(expected, Some(&observed)) {
                return Err(Error::MetadataChanged);
            }
            self.pinned_views()?;
        }
        Ok(())
    }

    fn pinned_edges<G: FnMut() -> Result<(), String>>(&self, guard: &mut G) -> Result<(), Error> {
        for link in &self.links {
            guard().map_err(Error::Guard)?;
            let current = original_pointer(link.parameter.parameter.raw());
            guard().map_err(Error::Guard)?;
            if current != link.original.raw() {
                return Err(Error::MetadataChanged);
            }
            let expected = match &link.owner {
                Some(owner) => unsafe { owner.as_borrowed_unknown() }?.as_ptr(),
                None => std::ptr::null_mut(),
            };
            let current = controller_pointer(link.parameter.parameter.raw());
            guard().map_err(Error::Guard)?;
            if current != expected {
                return Err(Error::MetadataChanged);
            }
        }
        // The outer guard may release temporary route owners. Compare the
        // already pinned borrowed edges again after its last callback.
        guard().map_err(Error::Guard)?;
        self.pinned_views()
    }

    fn pinned_views(&self) -> Result<(), Error> {
        for link in &self.links {
            let expected_owner = match &link.owner {
                Some(owner) => unsafe { owner.as_borrowed_unknown() }?.as_ptr(),
                None => std::ptr::null_mut(),
            };
            if original_pointer(link.parameter.parameter.raw()) != link.original.raw()
                || controller_pointer(link.parameter.parameter.raw()) != expected_owner
            {
                return Err(Error::MetadataChanged);
            }
        }
        Ok(())
    }

    fn capture<G: FnMut() -> Result<(), String>>(&mut self, guard: &mut G) -> Result<(), Error> {
        guard().map_err(Error::Guard)?;
        let mut next = hold_parameter(&self.parameter, self.parameter.parameter.raw())?;
        guard().map_err(Error::Guard)?;
        loop {
            let current_identity = identity(&next.parameter, guard)?;
            let owner = next.controller_owner(guard)?;
            guard().map_err(Error::Guard)?;
            let pointer = original_pointer(next.parameter.raw());
            // The borrowed getter must be retained before a reentrant guard.
            let original = unsafe { Owned::hold(pointer) }?;
            guard().map_err(Error::Guard)?;
            let original_identity = identity(&original, guard)?;
            let terminal = original_identity.raw() == current_identity.raw();
            let seen = self
                .links
                .iter()
                .any(|link| link.identity.raw() == original_identity.raw());
            self.links.push(Link {
                owner,
                original,
                original_identity,
                identity: current_identity,
                parameter: next,
            });
            guard().map_err(Error::Guard)?;
            if terminal {
                self.binding_complete = true;
                return self.check(guard);
            }
            if seen || self.links.len() == MAX_ORIGINAL_PARAMETERS {
                return Err(Error::InvalidData(
                    "numeric edit original chain is cyclic or incomplete",
                ));
            }
            let last = self.links.last().expect("captured original");
            next = hold_parameter(&self.parameter, last.original.raw())?;
            guard().map_err(Error::Guard)?;
        }
    }

    fn description<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<Description, Error> {
        self.check(guard)?;
        let result = self.parameter.describe();
        self.check(guard)?;
        result
    }

    fn capture_numeric<G: FnMut() -> Result<(), String>>(
        &mut self,
        guard: &mut G,
    ) -> Result<(), Error> {
        self.check(guard)?;
        let object = self.parameter.parameter.query(&ffi::OBJECT)?;
        self.numeric = Some(NumericAccess {
            object,
            ids: [std::ptr::null(); 3],
        });
        self.check(guard)?;
        for (index, name) in [c"value", c"min", c"max"].into_iter().enumerate() {
            let id = ffi::property_id(self.parameter.text, name)?;
            self.numeric.as_mut().expect("retained numeric access").ids[index] = id;
            self.check(guard)?;
        }
        Ok(())
    }

    fn writable<G: FnMut() -> Result<(), String>>(
        &self,
        before: &Description,
        number: Number,
        guard: &mut G,
    ) -> Result<(), GateError> {
        self.check(guard).map_err(GateError::Attachment)?;
        let observed = self.parameter.describe();
        self.check(guard).map_err(GateError::Attachment)?;
        let current = observed.map_err(GateError::Metadata)?;
        same_numeric_shape(before, &current).map_err(GateError::Metadata)?;
        self.profile
            .validate(&current)
            .map_err(GateError::Metadata)?;
        validate_number(&current, number).map_err(GateError::Metadata)?;
        self.check(guard).map_err(GateError::Attachment)?;
        self.parameter
            .final_guard(before, guard, None)
            .map_err(|error| {
                if matches!(error, Error::Guard(_)) {
                    GateError::Attachment(error)
                } else {
                    GateError::Metadata(error)
                }
            })?;
        self.pinned_edges(guard).map_err(GateError::Attachment)?;
        // The last route guard can change numeric metadata while releasing
        // owners. Successful scalar OUT reads release no native references.
        self.scalar_witness(before, number)
            .map_err(GateError::Metadata)?;
        self.pinned_views().map_err(GateError::Attachment)
    }

    fn scalar_witness(&self, before: &Description, number: Number) -> Result<NativeValue, Error> {
        let numeric = self
            .numeric
            .as_ref()
            .ok_or(Error::MissingInterface("numeric IObject"))?;
        let mut current = before.clone();
        current.raw = Some(numeric.scalar(0)?);
        current.min = Some(numeric.scalar(1)?);
        current.max = Some(numeric.scalar(2)?);
        let (name, tag, kind) = self.parameter.signature()?;
        current.name = name;
        current.tag = tag;
        current.kind = kind;
        (current.enabled, current.read_only) = self.parameter.flags();
        same_numeric_shape(before, &current)?;
        self.profile.validate(&current)?;
        validate_number(&current, number)?;
        current
            .raw
            .ok_or(Error::MissingValue("numeric edit scalar value"))
    }

    fn close_after_gate_error<G: FnMut() -> Result<(), String>>(
        &mut self,
        error: GateError,
        guard: &mut G,
    ) -> Error {
        let error = match error {
            GateError::Attachment(error) => return error,
            GateError::Metadata(error) => error,
        };
        // Context/attachment checks are independent of writable metadata. Only
        // an attached receiver may receive the single explicit balancing end.
        if let Err(attachment_error) = self.check(guard) {
            return attachment_error;
        }
        self.phase = NumericEditPhase::EndDispatched;
        self.dispatch_lifecycle(17);
        match self.check(guard) {
            Ok(()) => error,
            Err(attachment_error) => attachment_error,
        }
    }

    fn dispatch_lifecycle(&self, slot: usize) {
        let call: unsafe extern "C" fn(*mut c_void) =
            unsafe { ffi::method(self.parameter.parameter.raw(), slot) };
        unsafe { call(self.parameter.parameter.raw()) };
    }
}

impl NumericAccess {
    fn scalar(&self, index: usize) -> Result<NativeValue, Error> {
        let mut output = Variant::empty();
        let get: unsafe extern "C" fn(*mut c_void, *mut Variant, *const c_void) -> u8 =
            unsafe { ffi::method(self.object.raw(), 4) };
        let accepted = unsafe { get(self.object.raw(), &mut output, self.ids[index]) } != 0;
        if !accepted {
            return Err(Error::MissingValue("numeric edit scalar property"));
        }
        if !matches!(output.tag & 15, 1 | 2) || output.tag & 0x10 != 0 {
            return Err(Error::TypeMismatch);
        }
        let copied = output.decode()?;
        value::number(Some(&copied)).ok_or(Error::NonFinite)?;
        Ok(copied)
    }
}

enum GateError {
    Attachment(Error),
    Metadata(Error),
}
impl GateError {
    fn error(self) -> Error {
        match self {
            Self::Attachment(error) | Self::Metadata(error) => error,
        }
    }
}

fn same_numeric_shape(before: &Description, after: &Description) -> Result<(), Error> {
    if before.name != after.name
        || before.tag != after.tag
        || before.kind != after.kind
        || before.min != after.min
        || before.max != after.max
        || before.raw.as_ref().map(|v| v.variant_tag) != after.raw.as_ref().map(|v| v.variant_tag)
    {
        return Err(Error::MetadataChanged);
    }
    Ok(())
}
fn hold_parameter(source: &RawParameter, pointer: *mut c_void) -> Result<RawParameter, Error> {
    let owner = unsafe { Owned::hold(pointer) }?;
    Ok(unsafe {
        RawParameter::from_owned(
            NonNull::new(owner.into_raw()).expect("retained parameter"),
            source.text,
        )
    })
}
fn identity<G: FnMut() -> Result<(), String>>(
    owner: &Owned,
    guard: &mut G,
) -> Result<Owned, Error> {
    guard().map_err(Error::Guard)?;
    let result = owner.query_optional(&ffi::UNKNOWN, "IUnknown");
    guard().map_err(Error::Guard)?;
    result?.ok_or(Error::MissingInterface("IUnknown"))
}
fn check_identity<G: FnMut() -> Result<(), String>>(
    owner: &Owned,
    expected: &Owned,
    guard: &mut G,
) -> Result<(), Error> {
    let observation = identity(owner, guard);
    let matched = observation
        .as_ref()
        .is_ok_and(|identity| identity.raw() == expected.raw());
    let error = observation.as_ref().err().cloned();
    let before_release = guard().map_err(Error::Guard);
    drop(observation);
    let after_release = guard().map_err(Error::Guard);
    before_release.and(after_release)?;
    if let Some(error) = error {
        Err(error)
    } else if matched {
        Ok(())
    } else {
        Err(Error::MetadataChanged)
    }
}
fn original_pointer(parameter: *mut c_void) -> *mut c_void {
    let get: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
        unsafe { ffi::method(parameter, 18) };
    unsafe { get(parameter) }
}
fn controller_pointer(parameter: *mut c_void) -> *mut c_void {
    let get: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
        unsafe { ffi::method(parameter, 14) };
    unsafe { get(parameter) }
}
