//! Bounded, read-only parameter enumeration for one already resolved DSP target.
//!
//! The interface is an undocumented, exact-version Fender Studio Pro 8.1.1 ABI,
//! not a public CCL interface. Construction requires prior runtime validation.
//! No loader, target scan, IPC or native setter is included.
#![deny(unsafe_op_in_unsafe_fn)]

mod ffi;
mod record;

use ffi::Owned;
use std::{
    ffi::{c_char, c_void},
    ptr::NonNull,
    thread::{self, ThreadId},
};

pub const MAX_PAGE_SIZE: u32 = 128;
pub const MAX_PARAMETER_COUNT: u32 = 1_000_000;
pub const MAX_NAME_BYTES: usize = 512;
pub const MAX_TITLE_UNITS: usize = 512;

/// Already resolved public text constants from the validated CCL 5.1.2 runtime.
/// Both return borrowed references to 16-byte native String/CString objects.
#[derive(Clone, Copy)]
pub struct TextApi {
    pub get_constant_string: unsafe extern "C" fn(*const c_char) -> *const c_void,
    pub get_constant_cstring: unsafe extern "C" fn(*const c_char) -> *const c_void,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameter {
    pub index: u32,
    pub tag: u32,
    pub name: String,
    pub title: String,
    pub folder: u32,
    /// Uninterpreted native bits; not a writable capability declaration.
    pub raw_flags: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailedIndex {
    pub index: u32,
    pub reason: InfoError,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub reported_count: u32,
    pub offset: u32,
    pub attempted: u32,
    pub cap: u32,
    pub records: Vec<Parameter>,
    pub failed_indices: Vec<FailedIndex>,
    pub next: Option<u32>,
    /// True only if this call alone read every reported index successfully.
    /// A final page with offset > 0 cannot establish earlier pages' coverage.
    pub complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    WrongThread,
    MissingProvider(i32),
    NullProvider,
    InvalidCount(i32),
    InvalidLimit(u32),
    InvalidOffset { offset: u32, count: u32 },
    MissingTextConstant(&'static str),
    InvalidEmptyString,
    CountChanged { before: u32, after: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InfoError {
    NativeStatus(u8),
    InvalidStringOwners,
    NameTooLong,
    InvalidNameUtf8,
    TitleTooLong,
    TitleCopyFailed(i32),
    TitleChangedLength,
    TitleNotTerminated,
    InvalidTitleUtf16,
}

/// Owns a target IUnknown and the provider obtained through normal QueryInterface.
///
/// ```compile_fail
/// use fender_native_dsp_api::DspParameters;
/// fn worker(provider: DspParameters) {std::thread::spawn(move || drop(provider));}
/// ```
///
/// ```compile_fail
/// use fender_native_dsp_api::DspParameters;
/// fn require_sync<T: Sync>() {}
/// require_sync::<DspParameters>();
/// ```
pub struct DspParameters {
    // Drop the provider before its owning target reference.
    provider: Owned,
    _target: Owned,
    text: TextApi,
    thread: ThreadId,
}
impl DspParameters {
    /// Takes one caller-owned target IUnknown reference, including on error.
    ///
    /// # Safety
    /// Before this call, validate the exact Fender Studio Pro 8.1.1 arm64 main
    /// and CCL 5.1.2 cclgui runtime UUIDs, and the current GUI thread. `target`
    /// must be a live IUnknown interface with one owned reference transferred
    /// here. Text APIs must come from that runtime and outlive this wrapper.
    /// Retain the document/model/target context until this wrapper is dropped,
    /// and validate that context before and after each call. No native code
    /// may unwind through Rust. Use and drop this object on that GUI thread.
    pub unsafe fn from_owned_target(target: NonNull<c_void>, text: TextApi) -> Result<Self, Error> {
        let target = unsafe { Owned::take(target) };
        let provider = target.query_provider()?;
        Ok(Self {
            provider,
            _target: target,
            text,
            thread: thread::current().id(),
        })
    }
    fn check_thread(&self) -> Result<(), Error> {
        if self.thread == thread::current().id() {
            Ok(())
        } else {
            Err(Error::WrongThread)
        }
    }
    pub fn count(&self) -> Result<u32, Error> {
        self.check_thread()?;
        let get: unsafe extern "C" fn(*mut c_void) -> i32 =
            unsafe { ffi::method(self.provider.raw(), 3) };
        let count = unsafe { get(self.provider.raw()) };
        if !(0..=MAX_PARAMETER_COUNT as i32).contains(&count) {
            Err(Error::InvalidCount(count))
        } else {
            Ok(count as u32)
        }
    }
    /// Read at most 128 indices. Failures do not suppress later indices.
    /// `next` advances past attempted indices; retry failed indices explicitly.
    pub fn page(&self, offset: u32, limit: u32) -> Result<Page, Error> {
        self.check_thread()?;
        if !(1..=MAX_PAGE_SIZE).contains(&limit) {
            return Err(Error::InvalidLimit(limit));
        }
        let count = self.count()?;
        if offset > count {
            return Err(Error::InvalidOffset { offset, count });
        }
        let end = offset.saturating_add(limit).min(count);
        let mut records = Vec::with_capacity((end - offset) as usize);
        let mut failed_indices = Vec::new();
        if offset < end {
            let empty = record::EmptyValues::new(self.text)?;
            let info: unsafe extern "C" fn(*mut c_void, *mut record::ParamInfo, i32) -> u8 =
                unsafe { ffi::method(self.provider.raw(), 4) };
            for index in offset..end {
                let mut record = empty.record();
                let status = unsafe { info(self.provider.raw(), &mut record.0, index as i32) };
                let decoded = if status == 1 {
                    record.decode(index)
                } else {
                    Err(InfoError::NativeStatus(status))
                };
                // Explicit cleanup before returning copied data. Native output
                // assignment may have replaced all owners, even on failure.
                drop(record);
                match decoded {
                    Ok(parameter) => records.push(parameter),
                    Err(reason) => failed_indices.push(FailedIndex { index, reason }),
                }
            }
        }
        let after = self.count()?;
        if after != count {
            return Err(Error::CountChanged {
                before: count,
                after,
            });
        }
        Ok(Page {
            reported_count: count,
            offset,
            attempted: end - offset,
            cap: MAX_PAGE_SIZE,
            next: if end < count { Some(end) } else { None },
            complete: offset == 0 && end == count && failed_indices.is_empty(),
            records,
            failed_indices,
        })
    }
}

#[cfg(test)]
mod tests;
