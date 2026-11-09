use super::*;

impl RawParameter {
    /// Reads precision, fixed state flags and IListParameter QI capability.
    /// Precision is the host's increment/decrement value, not a choice count.
    /// The guard must validate the retained document and target context.
    pub fn details<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<ParameterDetails, Error> {
        self.check_thread()?;
        let raw = self.parameter.raw();
        let precision = observe(guard, || {
            let get: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { ffi::method(raw, 30) };
            Ok(unsafe { get(raw) })
        })?;
        let mut state = |mask| {
            observe(guard, || {
                let get: unsafe extern "C" fn(*mut c_void, i32) -> u8 =
                    unsafe { ffi::method(raw, 8) };
                Ok(unsafe { get(raw, mask) } != 0)
            })
        };
        let storable = state(0x80)?;
        let store_list_value = state(0x100)?;
        let mutable = state(0x800)?;
        let read_only = state(0x1000)?;
        let can_undo = state(0x10000)?;
        guard().map_err(Error::Guard)?;
        let list = self
            .parameter
            .query_optional(&ffi::LIST_PARAMETER, "IListParameter");
        let after_query = guard().map_err(Error::Guard);
        let list_interface = list
            .as_ref()
            .map(|value| value.is_some())
            .map_err(Clone::clone);
        let released = release(list, guard);
        after_query?;
        released?;
        Ok(ParameterDetails {
            precision,
            storable,
            mutable,
            store_list_value,
            read_only,
            can_undo,
            list_interface: list_interface?,
        })
    }

    /// Observes up to eight distinct canonical parameters, including self.
    /// getOriginal is borrowed, so each nonnull return is retained before the
    /// caller's guard can reenter. Canonical IUnknown pins prevent pointer reuse.
    /// All temporary references are released before the final context check.
    /// No original pointer or mutation capability is returned.
    pub fn original_chain<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<OriginalChain, Error> {
        self.check_thread()?;
        let mut originals = Vec::new();
        let mut identities = Vec::new();
        let result = (|| {
            guard().map_err(Error::Guard)?;
            let identity = self.parameter.query_optional(&ffi::UNKNOWN, "IUnknown");
            if let Ok(Some(identity)) = identity {
                identities.push(identity);
            } else {
                guard().map_err(Error::Guard)?;
                return Err(identity
                    .err()
                    .unwrap_or(Error::MissingInterface("IUnknown")));
            }
            guard().map_err(Error::Guard)?;
            let mut parameters = vec![self.original_observation(guard)?];
            loop {
                if parameters.len() == MAX_ORIGINAL_PARAMETERS {
                    return Ok(OriginalChain {
                        parameters,
                        end: OriginalChainEnd::Limit,
                    });
                }
                let current = originals.last().unwrap_or(self);
                guard().map_err(Error::Guard)?;
                let get: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
                    unsafe { ffi::method(current.parameter.raw(), 18) };
                let pointer = unsafe { get(current.parameter.raw()) };
                if pointer.is_null() {
                    guard().map_err(Error::Guard)?;
                    return Ok(OriginalChain {
                        parameters,
                        end: OriginalChainEnd::MissingOriginal,
                    });
                }
                // Do not insert a guard between the borrowed return and retain.
                let held = unsafe { Owned::hold(pointer) }?;
                originals.push(unsafe {
                    RawParameter::from_owned(
                        NonNull::new(held.into_raw()).expect("held parameter is nonnull"),
                        self.text,
                    )
                });
                guard().map_err(Error::Guard)?;
                let original = originals.last().expect("original just retained");
                guard().map_err(Error::Guard)?;
                let identity = original.parameter.query_optional(&ffi::UNKNOWN, "IUnknown");
                if let Ok(Some(identity)) = identity {
                    identities.push(identity);
                } else {
                    guard().map_err(Error::Guard)?;
                    return Err(identity
                        .err()
                        .unwrap_or(Error::MissingInterface("IUnknown")));
                }
                guard().map_err(Error::Guard)?;
                let canonical = identities.last().expect("identity just retained").raw();
                let current_index = parameters.len() - 1;
                let seen = identities[..identities.len() - 1]
                    .iter()
                    .position(|identity| identity.raw() == canonical);
                if let Some(index) = seen {
                    return Ok(OriginalChain {
                        parameters,
                        end: if index == current_index {
                            OriginalChainEnd::SelfReference
                        } else {
                            OriginalChainEnd::Cycle { index }
                        },
                    });
                }
                parameters.push(original.original_observation(guard)?);
            }
        })();
        let released_originals = release_all(originals, guard);
        let released_identities = release_all(identities, guard);
        let final_guard = guard().map_err(Error::Guard);
        released_originals?;
        released_identities?;
        final_guard?;
        result
    }

    fn original_observation<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<OriginalParameter, Error> {
        // describe returns only owned Rust data. Its native temporaries have
        // finished releasing before observe performs the following guard.
        let description = observe(guard, || self.describe())?;
        let details = self.details(guard)?;
        Ok(OriginalParameter {
            description,
            details,
        })
    }
}

fn observe<T, G: FnMut() -> Result<(), String>>(
    guard: &mut G,
    read: impl FnOnce() -> Result<T, Error>,
) -> Result<T, Error> {
    guard().map_err(Error::Guard)?;
    let result = read();
    guard().map_err(Error::Guard)?;
    result
}

fn release<T, G: FnMut() -> Result<(), String>>(value: T, guard: &mut G) -> Result<(), Error> {
    let before = guard().map_err(Error::Guard);
    drop(value);
    let after = guard().map_err(Error::Guard);
    before.and(after)
}

fn release_all<T, G: FnMut() -> Result<(), String>>(
    mut owners: Vec<T>,
    guard: &mut G,
) -> Result<(), Error> {
    let mut result = Ok(());
    while let Some(owner) = owners.pop() {
        let released = release(owner, guard);
        if result.is_ok() {
            result = released;
        }
    }
    result
}
