use super::{
    Error, NativeValue, Number, TextApi, Value,
    ffi::{self, Owned},
    text,
};
use std::{
    cmp::Ordering,
    ffi::{CStr, c_void},
};

/// Non-Copy/non-Clone ownership carrier for C++ Variant references only.
#[repr(C)]
pub(crate) struct Variant {
    pub tag: i16,
    pub padding: [u8; 6],
    pub payload: u64,
}
impl Variant {
    pub(crate) fn empty() -> Self {
        Self {
            tag: 0,
            padding: [0; 6],
            payload: 0,
        }
    }
    pub(crate) fn number(number: Number, tag: i16) -> Result<Self, Error> {
        let payload = match (number, tag & 15) {
            (Number::Integer(n), 1) => n as u64,
            (Number::Float(n), 2) if n.is_finite() => n.to_bits(),
            (Number::Float(_), 2) => return Err(Error::NonFinite),
            _ => return Err(Error::TypeMismatch),
        };
        Ok(Self {
            tag,
            padding: [0; 6],
            payload,
        })
    }
    pub(crate) fn decode(&self) -> Result<NativeValue, Error> {
        let value = match self.tag & 15 {
            0 => Value::Empty,
            1 => Value::Integer(self.payload as i64),
            2 => {
                let n = f64::from_bits(self.payload);
                if n.is_finite() {
                    Value::Float(n)
                } else {
                    Value::NonFiniteFloat { bits: self.payload }
                }
            }
            3 => Value::Text(text::copy_i_string(self.payload as usize as *mut c_void)?),
            4 => Value::OpaqueObject {
                present: self.payload != 0,
            },
            kind => Value::Unsupported { kind },
        };
        Ok(NativeValue {
            variant_tag: self.tag,
            value,
        })
    }
}
impl Drop for Variant {
    fn drop(&mut self) {
        if let (true, Some(pointer)) = (
            self.tag & 0x10 != 0 && matches!(self.tag & 15, 3 | 4),
            std::ptr::NonNull::new(self.payload as usize as *mut c_void),
        ) {
            drop(unsafe { Owned::take(pointer) });
        }
        self.tag = 0;
        self.payload = 0;
    }
}
#[derive(Clone, Copy)]
pub(crate) enum Property {
    Value,
    Min,
    Max,
    Default,
}
impl Property {
    fn name(self) -> &'static CStr {
        match self {
            Self::Value => c"value",
            Self::Min => c"min",
            Self::Max => c"max",
            Self::Default => c"default",
        }
    }
}
pub(crate) fn read(
    object: &Owned,
    api: TextApi,
    property: Property,
) -> Result<Option<NativeValue>, Error> {
    let id = ffi::property_id(api, property.name())?;
    let get: unsafe extern "C" fn(*mut c_void, *mut Variant, *const c_void) -> u8 =
        unsafe { ffi::method(object.raw(), 4) };
    let mut out = Variant::empty();
    if unsafe { get(object.raw(), &mut out, id) } == 0 {
        return Ok(None);
    }
    out.decode().map(Some)
}
pub(crate) fn number(value: Option<&NativeValue>) -> Option<Number> {
    match &value?.value {
        Value::Integer(n) => Some(Number::Integer(*n)),
        Value::Float(n) => Some(Number::Float(*n)),
        _ => None,
    }
}
pub(crate) fn finite(n: Number) -> Result<Number, Error> {
    if matches!(n,Number::Float(v) if !v.is_finite()) {
        Err(Error::NonFinite)
    } else {
        Ok(n)
    }
}
fn cmp_integer_float(integer: i64, float: f64) -> Ordering {
    // Exact comparison without rounding a large integer through f64.
    if float < i64::MIN as f64 {
        return Ordering::Greater;
    }
    if float >= 9223372036854775808.0 {
        return Ordering::Less;
    }
    let truncated = float as i64;
    match integer.cmp(&truncated) {
        Ordering::Equal => {
            if float.fract() > 0.0 {
                Ordering::Less
            } else if float.fract() < 0.0 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }
        other => other,
    }
}
pub(crate) fn compare(left: Number, right: Number) -> Ordering {
    match (left, right) {
        (Number::Integer(a), Number::Integer(b)) => a.cmp(&b),
        (Number::Float(a), Number::Float(b)) => {
            if a == b {
                Ordering::Equal
            } else {
                a.total_cmp(&b)
            }
        }
        (Number::Integer(a), Number::Float(b)) => cmp_integer_float(a, b),
        (Number::Float(a), Number::Integer(b)) => cmp_integer_float(b, a).reverse(),
    }
}
pub(crate) fn in_range(value: Number, min: Number, max: Number) -> Result<(), Error> {
    finite(value)?;
    finite(min)?;
    finite(max)?;
    if compare(min, max) == Ordering::Greater
        || compare(value, min) == Ordering::Less
        || compare(value, max) == Ordering::Greater
    {
        Err(Error::OutOfRange)
    } else {
        Ok(())
    }
}
pub(crate) fn coerce(value: Number, native: Number) -> Result<Number, Error> {
    finite(value)?;
    match (value, native) {
        (Number::Integer(n), Number::Float(_)) => {
            let f = n as f64;
            if f as i128 != n as i128 {
                Err(Error::TypeMismatch)
            } else {
                Ok(Number::Float(f))
            }
        }
        (Number::Float(f), Number::Integer(_)) => {
            if f.fract() != 0.0 || !(-9223372036854775808.0..9223372036854775808.0).contains(&f) {
                Err(Error::TypeMismatch)
            } else {
                Ok(Number::Integer(f as i64))
            }
        }
        (n, _) => Ok(n),
    }
}
pub(crate) fn exact(expected: Number, value: Option<&NativeValue>) -> bool {
    match (expected, number(value)) {
        (Number::Integer(a), Some(Number::Integer(b))) => a == b,
        (Number::Float(a), Some(Number::Float(b))) => a.to_bits() == b.to_bits(),
        _ => false,
    }
}
const _: () = {
    assert!(std::mem::size_of::<Variant>() == 16);
    assert!(std::mem::align_of::<Variant>() == 8);
    assert!(std::mem::offset_of!(Variant, tag) == 0);
    assert!(std::mem::offset_of!(Variant, payload) == 8);
};
