//! Owned CCL 5.1.2 parameter data access. No resolver, IPC, service or loader.
//!
//! The caller verifies the exact native ABI and GUI thread before creating an
//! owned parameter, and supplies its document/target context guard for writes.
//! No CCL implementation or header is copied into this library.
#![deny(unsafe_op_in_unsafe_fn)]

mod ffi;
mod parameter;
mod text;
mod value;

pub use parameter::RawParameter;
use std::ffi::{c_char, c_void};

pub const CCL_ABI_VERSION: &str = "5.1.2";
pub const MAX_TEXT_UNITS: usize = 4096;
pub const MAX_NAME_BYTES: usize = 512;
pub const MAX_CHOICES_PAGE: u32 = 128;
pub const MAX_LIST_CHOICES: u32 = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListChoice {
    pub index: u32,
    pub label: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListChoices {
    pub count: u32,
    pub offset: u32,
    pub next: Option<u32>,
    pub choices: Vec<ListChoice>,
}

/// Function pointers supplied by an already validated native host integration.
/// The returned pointers are borrowed CCL IString and const CString references.
#[derive(Clone, Copy)]
pub struct TextApi {
    pub get_empty_string: unsafe extern "C" fn() -> *mut c_void,
    pub get_constant_cstring: unsafe extern "C" fn(*const c_char) -> *const c_void,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Empty,
    Integer(i64),
    Float(f64),
    Text(String),
    NonFiniteFloat { bits: u64 },
    OpaqueObject { present: bool },
    Unsupported { kind: i16 },
}
#[derive(Clone, Debug, PartialEq)]
pub struct NativeValue {
    pub variant_tag: i16,
    pub value: Value,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Number {
    Integer(i64),
    Float(f64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Toggle,
    Integer,
    Float,
    String,
    List,
    Command,
    Color,
    Image,
    Segments,
    Scroll,
    TextModel,
    Unknown(i32),
}
impl Kind {
    fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Toggle,
            1 => Self::Integer,
            2 => Self::Float,
            3 => Self::String,
            4 => Self::List,
            5 => Self::Command,
            6 => Self::Color,
            7 => Self::Image,
            8 => Self::Segments,
            9 => Self::Scroll,
            10 => Self::TextModel,
            n => Self::Unknown(n),
        }
    }
    fn numeric(self) -> bool {
        matches!(
            self,
            Self::Toggle | Self::Integer | Self::Float | Self::List | Self::Scroll
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Normalized {
    NotApplicable,
    Finite(f32),
    NonFinite { bits: u32 },
}
#[derive(Clone, Debug, PartialEq)]
pub struct Description {
    pub name: String,
    pub tag: i32,
    pub kind: Kind,
    pub enabled: bool,
    pub read_only: bool,
    pub raw: Option<NativeValue>,
    pub min: Option<NativeValue>,
    pub max: Option<NativeValue>,
    pub default_value: Option<NativeValue>,
    pub normalized: Normalized,
    pub display: String,
    pub display_parser: DisplayParser,
    pub support: Support,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayParser {
    Unsupported,
    LiteralString,
    PlainNumeric,
    NativeFormatter { flags: i32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Support {
    pub raw_number: bool,
    pub normalized: bool,
    pub display_text: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    RawNumber(Number),
    Normalized(f32),
    DisplayText(String),
}
#[derive(Clone, Debug, PartialEq)]
pub struct MutationReadback {
    pub request: Request,
    pub before: Description,
    pub after: Description,
    /// True only when the observed raw, normalized or parsed-text target matches.
    /// Ok with false is a completed observation, not a verified requested value.
    pub verified: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    NotApplied,
    Unknown,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MutationError {
    pub outcome: Outcome,
    pub error: Error,
    pub before: Option<Box<Description>>,
    pub after: Option<Box<Description>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    WrongThread,
    MissingInterface(&'static str),
    NativeFailure(&'static str, i32),
    InvalidData(&'static str),
    TextTooLong,
    InvalidText,
    MissingValue(&'static str),
    Disabled,
    ReadOnly,
    UnsupportedKind,
    UnsupportedDisplayParser,
    TypeMismatch,
    OutOfRange,
    NonFinite,
    Guard(String),
    MetadataChanged,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl std::fmt::Display for MutationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.outcome, self.error)
    }
}
impl std::error::Error for MutationError {}

#[cfg(test)]
mod tests;
