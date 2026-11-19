use crate::{
    ffi::{self, Owned},
    *,
};
use std::{ffi::c_void, mem::ManuallyDrop};

pub const MAX_STRUCTURED_CHILDREN: usize = 32;
pub const MAX_STRUCTURED_TEXT_BYTES: usize = 32768;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructurePhase {
    BeforePrepare,
    PrepareDispatched,
    Prepared,
    CleanupDispatched,
    ClosedChecked,
    Unknown,
}
impl StructurePhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BeforePrepare => "before_prepare",
            Self::PrepareDispatched => "prepare_dispatched",
            Self::Prepared => "prepared",
            Self::CleanupDispatched => "cleanup_dispatched",
            Self::ClosedChecked => "closed_checked",
            Self::Unknown => "unknown",
        }
    }
    pub fn began(self) -> bool {
        self != Self::BeforePrepare
    }
}
#[derive(Debug)]
pub struct StructureObservation {
    pub parent_before: Description,
    pub parent_after: Description,
    pub parent_details: ParameterDetails,
    pub child_count: i32,
    pub children: Vec<Description>,
    pub observation_complete: bool,
    pub retained_text_bytes: usize,
}
pub struct StructureAttempt {
    pub phase: StructurePhase,
    pub prepare_calls: u32,
    pub cleanup_calls: u32,
    pub cleanup_returned: bool,
    pub cleanup_elapsed_ms: u64,
    pub result: Result<StructureObservation, Error>,
}
struct Resources {
    structured: Option<Owned>,
    parent_identity: Option<Owned>,
    children: Vec<RawParameter>,
    identities: Vec<Owned>,
}
fn same_description(left: &Description, right: &Description) -> bool {
    let values = [
        (&left.raw, &right.raw),
        (&left.min, &right.min),
        (&left.max, &right.max),
        (&left.default_value, &right.default_value),
    ];
    left == right
        && values.into_iter().all(|(a, b)| {
            match (a.as_ref().map(|v| &v.value), b.as_ref().map(|v| &v.value)) {
                (Some(Value::Float(a)), Some(Value::Float(b))) => a.to_bits() == b.to_bits(),
                _ => true,
            }
        })
        && match (left.normalized, right.normalized) {
            (Normalized::Finite(a), Normalized::Finite(b)) => a.to_bits() == b.to_bits(),
            _ => true,
        }
}
fn text_bytes(description: &Description) -> usize {
    description.name.len()
        + description.display.len()
        + [
            &description.raw,
            &description.min,
            &description.max,
            &description.default_value,
        ]
        .into_iter()
        .filter_map(|v| v.as_ref())
        .map(|v| match &v.value {
            Value::Text(s) => s.len(),
            _ => 0,
        })
        .sum::<usize>()
}
fn count(pointer: *mut c_void) -> i32 {
    let get: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { ffi::method(pointer, 5) };
    unsafe { get(pointer) }
}
fn child_at(pointer: *mut c_void, index: usize) -> *mut c_void {
    let get: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void =
        unsafe { ffi::method(pointer, 6) };
    unsafe { get(pointer, index as i32) }
}
fn remember(first: &mut Option<Error>, result: Result<(), Error>) {
    if let Err(error) = result {
        if first.is_none() {
            *first = Some(error);
        }
    }
}
fn checked<G: FnMut() -> Result<(), String>>(guard: &mut G) -> Result<(), Error> {
    guard().map_err(Error::Guard)
}
fn release_checked<T, G: FnMut() -> Result<(), String>>(
    owner: T,
    guard: &mut G,
    first: &mut Option<Error>,
) {
    remember(first, checked(guard));
    drop(owner);
    remember(first, checked(guard));
}

impl RawParameter {
    /// Materializes one direct structure on the retained original parameter.
    /// The caller keeps the original runtime, GUI thread and retained context alive.
    /// Guard failure invalidates observations but never substitutes a new cleanup target.
    pub fn inspect_structure<G: FnMut() -> Result<(), String>>(
        &self,
        deadline: std::time::Instant,
        guard: &mut G,
    ) -> StructureAttempt {
        let mut finite_guard = || {
            if std::time::Instant::now() > deadline {
                return Err("native request exceeded its two second budget".into());
            }
            guard()?;
            if std::time::Instant::now() > deadline {
                return Err("native request exceeded its two second budget".into());
            }
            Ok(())
        };
        let guard = &mut finite_guard;
        let mut attempt = StructureAttempt {
            phase: StructurePhase::BeforePrepare,
            prepare_calls: 0,
            cleanup_calls: 0,
            cleanup_returned: false,
            cleanup_elapsed_ms: 0,
            result: Err(Error::InvalidData("structure observation did not complete")),
        };
        let mut resources = ManuallyDrop::new(Resources {
            structured: None,
            parent_identity: None,
            children: vec![],
            identities: vec![],
        });
        let mut count_before = 0;
        let mut parent_before = None;
        let mut details_before = None;
        let mut retained_text_bytes = 0;
        let mut copied = Vec::new();
        let mut borrowed = Vec::new();
        let result = (|| -> Result<(), Error> {
            self.check_thread()?;
            checked(guard)?;
            let before = self.describe()?;
            checked(guard)?;
            if before.name != "portAssignment" || before.tag != 100 || before.kind != Kind::List {
                return Err(Error::InvalidData(
                    "fixed Output01 parameter signature mismatch",
                ));
            }
            let details = self.details(guard)?;
            resources.parent_identity = self.parameter.query_optional(&ffi::UNKNOWN, "IUnknown")?;
            checked(guard)?;
            if resources.parent_identity.is_none() {
                return Err(Error::MissingInterface("IUnknown"));
            }
            resources.structured = self
                .parameter
                .query_optional(&ffi::STRUCTURED_PARAMETER, "IStructuredParameter")?;
            checked(guard)?;
            let structured = resources
                .structured
                .as_ref()
                .ok_or(Error::MissingInterface("IStructuredParameter"))?
                .raw();
            parent_before = Some(before);
            details_before = Some(details);
            checked(guard)?;
            self.check_structure_header(
                parent_before.as_ref().expect("prepared parent description"),
            )?;
            attempt.phase = StructurePhase::PrepareDispatched;
            attempt.prepare_calls = 1;
            let prepare: unsafe extern "C" fn(*mut c_void) = unsafe { ffi::method(structured, 3) };
            unsafe { prepare(structured) };
            attempt.phase = StructurePhase::Prepared;
            checked(guard)?;
            count_before = count(structured);
            checked(guard)?;
            if count_before < 0 {
                return Err(Error::InvalidData("negative structured child count"));
            }
            if count_before as usize > MAX_STRUCTURED_CHILDREN {
                return Ok(());
            }
            for index in 0..count_before as usize {
                checked(guard)?;
                let pointer = child_at(structured, index);
                // The public getSubParameter result is borrowed until cleanup.
                let held = unsafe { Owned::hold(pointer) }?;
                let parameter = unsafe {
                    RawParameter::from_owned(
                        std::ptr::NonNull::new(held.into_raw()).expect("held child"),
                        self.text,
                    )
                };
                resources.children.push(parameter);
                checked(guard)?;
                let parameter = resources.children.last().expect("retained child");
                let identity = parameter
                    .parameter
                    .query_optional(&ffi::UNKNOWN, "IUnknown")?;
                let identity = identity.ok_or(Error::MissingInterface("IUnknown"))?;
                resources.identities.push(identity);
                checked(guard)?;
                let identity = resources.identities.last().expect("held identity");
                if pointer == self.parameter.raw()
                    || identity.raw()
                        == resources
                            .parent_identity
                            .as_ref()
                            .expect("parent identity")
                            .raw()
                    || resources.identities[..resources.identities.len() - 1]
                        .iter()
                        .any(|prior| prior.raw() == identity.raw())
                {
                    return Err(Error::InvalidData(
                        "null, self or duplicate structured child",
                    ));
                }
                let description = resources
                    .children
                    .last()
                    .expect("retained child")
                    .describe();
                checked(guard)?;
                let description = description?;
                let bytes = text_bytes(&description);
                if retained_text_bytes + bytes > MAX_STRUCTURED_TEXT_BYTES {
                    return Err(Error::TextTooLong);
                }
                retained_text_bytes += bytes;
                borrowed.push(pointer);
                copied.push(description.clone());
                checked(guard)?;
            }
            if count(structured) != count_before {
                return Err(Error::MetadataChanged);
            }
            checked(guard)?;
            for (index, child) in resources.children.iter().enumerate() {
                if child_at(structured, index) != child.parameter.raw() {
                    return Err(Error::MetadataChanged);
                }
                checked(guard)?;
                let identity = child.parameter.query_optional(&ffi::UNKNOWN, "IUnknown")?;
                let same = identity
                    .as_ref()
                    .is_some_and(|identity| identity.raw() == resources.identities[index].raw());
                let mut release_error = None;
                release_checked(identity, guard, &mut release_error);
                if let Some(error) = release_error {
                    return Err(error);
                }
                if !same {
                    return Err(Error::MetadataChanged);
                }
                let description = child.describe();
                checked(guard)?;
                if !same_description(&description?, &copied[index]) {
                    return Err(Error::MetadataChanged);
                }
            }
            Ok(())
        })();
        let mut first = result.err();
        // Explicit finalizer. No Drop implementation dispatches lifecycle callbacks.
        while let Some(identity) = resources.identities.pop() {
            release_checked(identity, guard, &mut first);
        }
        while let Some(child) = resources.children.pop() {
            release_checked(child, guard, &mut first);
        }
        if attempt.phase == StructurePhase::Prepared {
            if first.is_none() {
                let tail = (|| {
                    checked(guard)?;
                    let raw = resources
                        .structured
                        .as_ref()
                        .expect("prepared interface")
                        .raw();
                    if count(raw) != count_before {
                        return Err(Error::MetadataChanged);
                    }
                    for (index, pointer) in borrowed.iter().enumerate() {
                        if std::time::Instant::now() > deadline {
                            return Err(Error::Guard(
                                "native request exceeded its two second budget".into(),
                            ));
                        }
                        if child_at(raw, index) != *pointer {
                            return Err(Error::MetadataChanged);
                        }
                    }
                    Ok(())
                })();
                remember(&mut first, tail);
            }
            attempt.phase = StructurePhase::CleanupDispatched;
            attempt.cleanup_calls = 1;
            let raw = resources
                .structured
                .as_ref()
                .expect("prepared original interface")
                .raw();
            let cleanup: unsafe extern "C" fn(*mut c_void) = unsafe { ffi::method(raw, 4) };
            let cleanup_started = std::time::Instant::now();
            unsafe { cleanup(raw) };
            attempt.cleanup_elapsed_ms = cleanup_started
                .elapsed()
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX);
            attempt.cleanup_returned = true;
            remember(&mut first, checked(guard));
        }
        release_checked(resources.structured.take(), guard, &mut first);
        release_checked(resources.parent_identity.take(), guard, &mut first);
        if first.is_none() && attempt.cleanup_returned {
            let final_result = (|| {
                checked(guard)?;
                let after = self.describe()?;
                checked(guard)?;
                let details = self.details(guard)?;
                checked(guard)?;
                if parent_before
                    .as_ref()
                    .is_none_or(|before| !same_description(before, &after))
                    || details_before.as_ref() != Some(&details)
                {
                    return Err(Error::MetadataChanged);
                }
                self.check_structure_header(&after)?;
                Ok(after)
            })();
            match final_result {
                Ok(after) => {
                    attempt.phase = StructurePhase::ClosedChecked;
                    attempt.result = Ok(StructureObservation {
                        parent_before: parent_before.expect("prepared parent"),
                        parent_after: after,
                        parent_details: details_before.expect("prepared details"),
                        child_count: count_before,
                        children: copied,
                        observation_complete: count_before as usize <= MAX_STRUCTURED_CHILDREN,
                        retained_text_bytes,
                    });
                }
                Err(error) => first = Some(error),
            }
        }
        if let Some(error) = first {
            if attempt.phase.began() {
                attempt.phase = StructurePhase::Unknown;
            }
            attempt.result = Err(error);
        }
        unsafe { ManuallyDrop::drop(&mut resources) };
        attempt
    }
}
