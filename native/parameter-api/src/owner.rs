use super::*;

struct Observation {
    controller: Owned,
    canonical: Owned,
}

/// A retained, fixed-view parameter/controller association on its creator thread.
/// The caller keeps the validated runtime and document/provider context alive
/// through use and release, and rechecks that context before publication.
/// No controller interface beyond IUnknown is assumed.
///
/// ```compile_fail
/// use fender_native_parameter_api::ControllerOwner;
/// fn send_to_worker(owner: ControllerOwner) {
///     std::thread::spawn(move || drop(owner));
/// }
/// ```
///
/// ```compile_fail
/// use fender_native_parameter_api::ControllerOwner;
/// fn require_sync<T: Sync>() {}
/// require_sync::<ControllerOwner>();
/// ```
pub struct ControllerOwner {
    controller: Owned,
    canonical: Owned,
    parameter: Owned,
    thread: ThreadId,
}

impl RawParameter {
    /// Retains the parameter's declared controller without reflection or updates.
    /// Null observations are rechecked after temporary cleanup. A present
    /// controller is retained before reentry and its association is rechecked.
    pub fn controller_owner<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<Option<ControllerOwner>, Error> {
        self.check_thread()?;
        guard().map_err(Error::Guard)?;
        let parameter = unsafe { Owned::hold(self.parameter.raw()) }?;
        let observed = guard()
            .map_err(Error::Guard)
            .and_then(|_| observe(&parameter, guard));
        match observed {
            Ok(Some(observed)) => {
                let owner = ControllerOwner {
                    controller: observed.controller,
                    canonical: observed.canonical,
                    parameter,
                    thread: self.thread,
                };
                if let Err(error) = owner.check(guard) {
                    owner.release(guard)?;
                    return Err(error);
                }
                Ok(Some(owner))
            }
            result => {
                let result = match result {
                    Ok(None) => {
                        let after = observe(&parameter, guard);
                        let stable_null = matches!(after, Ok(None));
                        let error = after.as_ref().err().cloned();
                        let released = release_observation(after, guard);
                        if let Err(error) = released {
                            Err(error)
                        } else if let Some(error) = error {
                            Err(error)
                        } else if stable_null {
                            Ok(None)
                        } else {
                            Err(Error::MetadataChanged)
                        }
                    }
                    Err(error) => Err(error),
                    Ok(Some(_)) => unreachable!(),
                };
                release(parameter, guard)?;
                if result.is_ok() {
                    guard().map_err(Error::Guard)?;
                    let still_null = controller(self.parameter.raw()).is_null();
                    guard().map_err(Error::Guard)?;
                    if !still_null {
                        return Err(Error::MetadataChanged);
                    }
                }
                result
            }
        }
    }
}

impl ControllerOwner {
    fn check_thread(&self) -> Result<(), Error> {
        if thread::current().id() == self.thread {
            Ok(())
        } else {
            Err(Error::WrongThread)
        }
    }

    /// Revalidates the fixed controller view and its canonical identity.
    /// A replacement, missing owner or changed interface view is rejected.
    /// The caller's guard must not mutate the parameter or association.
    pub fn check<G: FnMut() -> Result<(), String>>(&self, guard: &mut G) -> Result<(), Error> {
        self.check_thread()?;
        let observed = observe(&self.parameter, guard);
        let result = match observed.as_ref() {
            Ok(Some(owner))
                if owner.controller.raw() == self.controller.raw()
                    && owner.canonical.raw() == self.canonical.raw() =>
            {
                Ok(())
            }
            Ok(_) => Err(Error::MetadataChanged),
            Err(error) => Err(error.clone()),
        };
        release_observation(observed, guard)?;
        result?;
        guard().map_err(Error::Guard)?;
        // The matching view already has a persistent reference. A mismatching
        // borrowed pointer is only compared, never dereferenced or published.
        let stable = controller(self.parameter.raw()) == self.controller.raw();
        guard().map_err(Error::Guard)?;
        if stable {
            Ok(())
        } else {
            Err(Error::MetadataChanged)
        }
    }

    /// Borrows the retained native IUnknown for a separately validated Rust API.
    ///
    /// # Safety
    /// Use only on the validated GUI thread after a successful association and
    /// context check. The pointer must not outlive this owner or its runtime and
    /// document/provider context. A receiving owner must retain it before any
    /// reentrant callback. It must never become a JSON pointer or invocation API.
    pub unsafe fn as_borrowed_unknown(&self) -> Result<NonNull<c_void>, Error> {
        self.check_thread()?;
        Ok(NonNull::new(self.controller.raw()).expect("retained controller is nonnull"))
    }

    /// Releases all persistent references between context guards.
    /// Keep the caller's context alive through this release. Ordinary Drop has
    /// the same creator-thread/context lifetime requirement as RawParameter.
    pub fn release<G: FnMut() -> Result<(), String>>(self, guard: &mut G) -> Result<(), Error> {
        let Self {
            controller,
            canonical,
            parameter,
            thread: _,
        } = self;
        let controller = release(controller, guard);
        let canonical = release(canonical, guard);
        let parameter = release(parameter, guard);
        controller.and(canonical).and(parameter)
    }
}

fn controller(parameter: *mut c_void) -> *mut c_void {
    let get: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
        unsafe { ffi::method(parameter, 14) };
    unsafe { get(parameter) }
}

fn observe<G: FnMut() -> Result<(), String>>(
    parameter: &Owned,
    guard: &mut G,
) -> Result<Option<Observation>, Error> {
    guard().map_err(Error::Guard)?;
    let pointer = controller(parameter.raw());
    let mut controller = if pointer.is_null() {
        None
    } else {
        // The declared getter is borrowed. Retain before the guard can reenter.
        Some(unsafe { Owned::hold(pointer) }?)
    };
    let mut canonical = None;
    let result = (|| {
        guard().map_err(Error::Guard)?;
        let Some(owner) = controller.as_ref() else {
            return Ok(());
        };
        guard().map_err(Error::Guard)?;
        let queried = owner.query_optional(&ffi::UNKNOWN, "IUnknown");
        let queried = match queried {
            Ok(owner) => {
                canonical = owner;
                Ok(())
            }
            Err(error) => Err(error),
        };
        guard().map_err(Error::Guard)?;
        queried?;
        if canonical.is_none() {
            return Err(Error::MissingInterface("IUnknown"));
        }
        Ok(())
    })();
    match result {
        Ok(()) => Ok(controller.take().map(|controller| Observation {
            controller,
            canonical: canonical
                .take()
                .expect("present controller has canonical identity"),
        })),
        Err(error) => {
            let canonical = release(canonical, guard);
            let controller = release(controller, guard);
            canonical.and(controller)?;
            Err(error)
        }
    }
}

fn release<T, G: FnMut() -> Result<(), String>>(owner: T, guard: &mut G) -> Result<(), Error> {
    let before = guard().map_err(Error::Guard);
    drop(owner);
    let after = guard().map_err(Error::Guard);
    before.and(after)
}

fn release_observation<G: FnMut() -> Result<(), String>>(
    observed: Result<Option<Observation>, Error>,
    guard: &mut G,
) -> Result<(), Error> {
    if let Ok(Some(Observation {
        controller,
        canonical,
    })) = observed
    {
        let controller = release(controller, guard);
        let canonical = release(canonical, guard);
        controller.and(canonical)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "owner_tests.rs"]
mod tests;
