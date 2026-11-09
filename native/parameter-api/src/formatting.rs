use super::*;

struct Snapshot {
    description: Description,
    details: ParameterDetails,
}

impl RawParameter {
    /// Formats 1..32 explicit raw numeric values without changing a selection.
    /// Order and duplicates are preserved. No domain or choice count is inferred.
    /// Inputs must fit the observed numeric range and current representation.
    /// Any context, metadata, exact current value or normalized-value change
    /// discards the entire observation, including already copied labels.
    pub fn format_values<G: FnMut() -> Result<(), String>>(
        &self,
        requested: &[Number],
        guard: &mut G,
    ) -> Result<FormattedValues, Error> {
        self.check_thread()?;
        if !(1..=MAX_FORMAT_VALUES).contains(&requested.len()) {
            return Err(Error::InvalidData(
                "format values must contain 1..32 numbers",
            ));
        }
        for value in requested {
            value::finite(*value)?;
        }
        let before = self.format_snapshot(guard)?;
        let (current, min, max) = numeric_metadata(&before.description)?;
        // Validate the whole batch before issuing any getString call.
        let values = requested
            .iter()
            .map(|requested| {
                let value = value::coerce(*requested, current)?;
                value::in_range(value, min, max)?;
                let tag = match value {
                    Number::Integer(_) => 1,
                    Number::Float(_) => 2,
                };
                Ok((*requested, Variant::number(value, tag)?))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let mut formatted = Vec::with_capacity(values.len());
        for (requested, value) in values {
            guard().map_err(Error::Guard)?;
            let mut output = NativeString::empty(self.text);
            let copied = (|| {
                guard().map_err(Error::Guard)?;
                let output = output.as_mut().map_err(|error| error.clone())?;
                let current = self.format_snapshot(guard)?;
                if !same_snapshot(&before, &current) {
                    return Err(Error::MetadataChanged);
                }
                guard().map_err(Error::Guard)?;
                let get: unsafe extern "C" fn(*mut c_void, *mut StringRepr, *const Variant) =
                    unsafe { ffi::method(self.parameter.raw(), 43) };
                unsafe { get(self.parameter.raw(), output.as_mut_ptr(), &value) };
                guard().map_err(Error::Guard)?;
                let copied = output.copy();
                guard().map_err(Error::Guard)?;
                copied
            })();
            let before_release = guard().map_err(Error::Guard);
            drop(output);
            let after_release = guard().map_err(Error::Guard);
            before_release?;
            after_release?;
            let label = copied?;
            let current = self.format_snapshot(guard)?;
            if !same_snapshot(&before, &current) {
                return Err(Error::MetadataChanged);
            }
            formatted.push(FormattedValue {
                requested,
                native_value: value.decode()?,
                label,
            });
        }
        let after = self.format_snapshot(guard)?;
        if !same_snapshot(&before, &after) {
            return Err(Error::MetadataChanged);
        }
        guard().map_err(Error::Guard)?;
        Ok(FormattedValues {
            values: formatted,
            before: before.description,
            after: after.description,
            current_unchanged: true,
        })
    }

    fn format_snapshot<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<Snapshot, Error> {
        guard().map_err(Error::Guard)?;
        let description = self.describe();
        // describe's native output owners finish releasing before this guard.
        guard().map_err(Error::Guard)?;
        let description = description?;
        if matches!(
            description.default_value.as_ref().map(|value| &value.value),
            Some(Value::OpaqueObject { .. } | Value::Unsupported { .. })
        ) {
            return Err(Error::InvalidData(
                "default value cannot be compared completely",
            ));
        }
        Ok(Snapshot {
            description,
            details: self.details(guard)?,
        })
    }
}

fn same_snapshot(left: &Snapshot, right: &Snapshot) -> bool {
    let a = &left.description;
    let b = &right.description;
    left.details == right.details
        && a.name == b.name
        && a.tag == b.tag
        && a.kind == b.kind
        && a.enabled == b.enabled
        && a.read_only == b.read_only
        && a.display == b.display
        && a.display_parser == b.display_parser
        && a.support == b.support
        && same_value(&a.raw, &b.raw)
        && same_value(&a.min, &b.min)
        && same_value(&a.max, &b.max)
        && same_value(&a.default_value, &b.default_value)
        && match (a.normalized, b.normalized) {
            (Normalized::Finite(a), Normalized::Finite(b)) => a.to_bits() == b.to_bits(),
            (a, b) => a == b,
        }
}

fn same_value(left: &Option<NativeValue>, right: &Option<NativeValue>) -> bool {
    match (left, right) {
        (Some(a), Some(b)) if a.variant_tag == b.variant_tag => match (&a.value, &b.value) {
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (a, b) => a == b,
        },
        (None, None) => true,
        _ => false,
    }
}
