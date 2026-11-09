//! Local fake vtables only. No DAW, loader, profile or application I/O.
use super::*;
use crate::{
    ffi::{CStringRepr, StringRepr, Uid},
    value::Variant,
};
use std::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    ptr::NonNull,
    rc::Rc,
    sync::OnceLock,
};

#[derive(Default)]
struct Stats {
    strings: Cell<u32>,
    destroyed: Cell<u32>,
}
#[repr(C)]
struct FakeString {
    table: *const usize,
    refs: Cell<u32>,
    units: RefCell<Vec<u16>>,
    stats: Rc<Stats>,
    fail_copy: Cell<bool>,
    stop_on_release: RefCell<Option<Rc<Cell<bool>>>>,
    final_release_only: Cell<bool>,
    skip_termination: Cell<bool>,
    stop_on_copy: RefCell<Option<Rc<Cell<bool>>>>,
}
fn table<const N: usize>(
    lock: &OnceLock<[usize; N]>,
    build: impl FnOnce() -> [usize; N],
) -> *const usize {
    lock.get_or_init(build).as_ptr()
}
fn string_new(text: &str, stats: Rc<Stats>) -> *mut c_void {
    static VT: OnceLock<[usize; 25]> = OnceLock::new();
    let table = table(&VT, || {
        let mut t = [0; 25];
        t[1] = string_retain as *const () as usize;
        t[2] = string_release as *const () as usize;
        t[4] = string_len as *const () as usize;
        t[8] = string_copy as *const () as usize;
        t[18] = string_clone as *const () as usize;
        t[21] = string_private as *const () as usize;
        t[24] = string_assign as *const () as usize;
        t
    });
    stats.strings.set(stats.strings.get() + 1);
    Box::into_raw(Box::new(FakeString {
        table,
        refs: Cell::new(1),
        units: RefCell::new(text.encode_utf16().collect()),
        stats,
        fail_copy: Cell::new(false),
        stop_on_release: RefCell::new(None),
        final_release_only: Cell::new(false),
        skip_termination: Cell::new(false),
        stop_on_copy: RefCell::new(None),
    }))
    .cast()
}
unsafe fn fs<'a>(p: *mut c_void) -> &'a FakeString {
    unsafe { &*p.cast() }
}
unsafe extern "C" fn string_retain(p: *mut c_void) -> u32 {
    let s = unsafe { fs(p) };
    let n = s.refs.get() + 1;
    s.refs.set(n);
    n
}
unsafe extern "C" fn string_release(p: *mut c_void) -> u32 {
    let s = unsafe { fs(p) };
    if !s.final_release_only.get() || s.refs.get() == 1 {
        if let Some(context) = s.stop_on_release.borrow().as_ref() {
            context.set(false);
        }
    }
    let n = s.refs.get() - 1;
    s.refs.set(n);
    if n == 0 {
        s.stats.destroyed.set(s.stats.destroyed.get() + 1);
        drop(unsafe { Box::from_raw(p.cast::<FakeString>()) });
    }
    n
}
unsafe extern "C" fn string_len(p: *mut c_void) -> i32 {
    unsafe { fs(p) }.units.borrow().len() as i32
}
unsafe extern "C" fn string_copy(p: *mut c_void, out: *mut u16, size: i32) -> i32 {
    let s = unsafe { fs(p) };
    if let Some(context) = s.stop_on_copy.borrow().as_ref() {
        context.set(false);
    }
    if s.fail_copy.get() {
        return -1;
    }
    let units = s.units.borrow();
    if size < units.len() as i32 + 1 {
        return -2;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(units.as_ptr(), out, units.len());
        if !s.skip_termination.get() {
            *out.add(units.len()) = 0;
        }
    }
    0
}
unsafe extern "C" fn string_clone(p: *mut c_void) -> *mut c_void {
    let s = unsafe { fs(p) };
    let text = String::from_utf16(&s.units.borrow()).unwrap_or_default();
    string_new(&text, s.stats.clone())
}
unsafe extern "C" fn string_private(_: *mut c_void) -> *mut c_void {
    std::ptr::null_mut()
}
unsafe extern "C" fn string_assign(p: *mut c_void, text: *const u16, n: i32) -> i32 {
    if n < 0 {
        return -1;
    }
    let s = unsafe { fs(p) };
    *s.units.borrow_mut() = unsafe { std::slice::from_raw_parts(text, n as usize) }.to_vec();
    0
}
thread_local! {
    static EMPTY:Cell<*mut c_void>=const{Cell::new(std::ptr::null_mut())};
    static VALUE_ID:CStringRepr=const{CStringRepr{text:c"value".as_ptr(),owner:std::ptr::null_mut()}};
    static MIN_ID:CStringRepr=const{CStringRepr{text:c"min".as_ptr(),owner:std::ptr::null_mut()}};
    static MAX_ID:CStringRepr=const{CStringRepr{text:c"max".as_ptr(),owner:std::ptr::null_mut()}};
    static DEFAULT_ID:CStringRepr=const{CStringRepr{text:c"default".as_ptr(),owner:std::ptr::null_mut()}};
}
unsafe extern "C" fn empty() -> *mut c_void {
    EMPTY.with(Cell::get)
}
unsafe extern "C" fn constant(name: *const std::ffi::c_char) -> *const c_void {
    match unsafe { std::ffi::CStr::from_ptr(name) }.to_bytes() {
        b"value" => VALUE_ID.with(|v| (v as *const CStringRepr).cast()),
        b"min" => MIN_ID.with(|v| (v as *const CStringRepr).cast()),
        b"max" => MAX_ID.with(|v| (v as *const CStringRepr).cast()),
        b"default" => DEFAULT_ID.with(|v| (v as *const CStringRepr).cast()),
        _ => std::ptr::null(),
    }
}
fn text_api() -> TextApi {
    TextApi {
        get_empty_string: empty,
        get_constant_cstring: constant,
    }
}
struct TextEnv {
    stats: Rc<Stats>,
    empty: *mut c_void,
}
impl TextEnv {
    fn new() -> Self {
        let stats = Rc::new(Stats::default());
        let empty = string_new("", stats.clone());
        EMPTY.with(|v| {
            assert!(v.get().is_null());
            v.set(empty)
        });
        Self { stats, empty }
    }
}
impl Drop for TextEnv {
    fn drop(&mut self) {
        EMPTY.with(|v| v.set(std::ptr::null_mut()));
        unsafe {
            string_release(self.empty);
        };
        assert_eq!(
            self.stats.strings.get(),
            self.stats.destroyed.get(),
            "all fake native strings released"
        );
    }
}

#[derive(Clone)]
enum FakeValue {
    Number(Number),
    Text(String),
    Object(*mut c_void),
    Empty,
}
#[repr(C)]
struct ObjectView {
    table: *const usize,
    parent: *mut FakeParameter,
}
#[repr(C)]
struct FakeParameter {
    table: *const usize,
    refs: Cell<u32>,
    object: ObjectView,
    name: CStringRepr,
    kind: i32,
    value: RefCell<FakeValue>,
    min: Number,
    max: Number,
    default: Number,
    enabled: Cell<bool>,
    readonly: Cell<bool>,
    writes: Cell<u32>,
    reads: Cell<u32>,
    context: Rc<Cell<bool>>,
    stop_on_enabled: Cell<bool>,
    stop_on_set: Cell<bool>,
    no_op_normalized: Cell<bool>,
    stop_on_formatter_release: Cell<bool>,
    stop_on_input_release: Cell<bool>,
    formatter: *mut c_void,
    quantize: bool,
    stats: Rc<Stats>,
    false_property: Cell<bool>,
    choices: RefCell<Vec<String>>,
    choice_calls: RefCell<Vec<i64>>,
    choice_copy_fails: Cell<bool>,
    choice_skip_termination: Cell<bool>,
    choice_invalid_utf16: Cell<bool>,
    choice_stop_on_get: Cell<bool>,
    choice_stop_on_copy: Cell<bool>,
    choice_stop_on_drop: Cell<bool>,
    choice_max_after_get: Cell<Option<Number>>,
    max_override: Cell<Option<Number>>,
}
unsafe fn fp<'a>(p: *mut c_void) -> &'a FakeParameter {
    unsafe { &*p.cast() }
}
unsafe extern "C" fn p_retain(p: *mut c_void) -> u32 {
    let p = unsafe { fp(p) };
    let n = p.refs.get() + 1;
    p.refs.set(n);
    n
}
unsafe extern "C" fn p_release(p: *mut c_void) -> u32 {
    let p = unsafe { fp(p) };
    let n = p.refs.get() - 1;
    p.refs.set(n);
    if n == 0 {
        if !p.formatter.is_null() {
            unsafe { f_release(p.formatter) };
        }
        drop(unsafe { Box::from_raw((p as *const FakeParameter).cast_mut()) });
    }
    n
}
unsafe extern "C" fn p_query(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    if unsafe { (*iid).a } != ffi::OBJECT.a {
        return -1;
    }
    let p = unsafe { fp(p) };
    unsafe {
        p_retain((p as *const FakeParameter).cast_mut().cast());
        *out = (&p.object as *const ObjectView).cast_mut().cast();
    }
    0
}
unsafe extern "C" fn o_retain(p: *mut c_void) -> u32 {
    unsafe { p_retain((*p.cast::<ObjectView>()).parent.cast()) }
}
unsafe extern "C" fn o_release(p: *mut c_void) -> u32 {
    unsafe { p_release((*p.cast::<ObjectView>()).parent.cast()) }
}
fn write_number(out: &mut Variant, n: Number) {
    *out = Variant::number(
        n,
        match n {
            Number::Integer(_) => 1,
            Number::Float(_) => 2,
        },
    )
    .unwrap();
}
unsafe extern "C" fn property(p: *mut c_void, out: *mut Variant, id: *const c_void) -> u8 {
    let p = unsafe { &*(*p.cast::<ObjectView>()).parent };
    p.reads.set(p.reads.get() + 1);
    let name = unsafe { std::ffi::CStr::from_ptr((*id.cast::<CStringRepr>()).text) }.to_bytes();
    let out = unsafe { &mut *out };
    match name {
        b"value" => match p.value.borrow().clone() {
            FakeValue::Number(n) => write_number(out, n),
            FakeValue::Text(s) => {
                out.tag = 0x13;
                out.payload = string_new(&s, p.stats.clone()) as usize as u64;
            }
            FakeValue::Object(o) => {
                out.tag = 0x14;
                out.payload = o as usize as u64;
                if !o.is_null() {
                    unsafe {
                        string_retain(o);
                    }
                }
            }
            FakeValue::Empty => {}
        },
        b"min" => write_number(out, p.min),
        b"max" => write_number(out, p.max_override.get().unwrap_or(p.max)),
        b"default" => write_number(out, p.default),
        _ => return 0,
    }
    u8::from(!p.false_property.get())
}
unsafe extern "C" fn p_kind(p: *mut c_void) -> i32 {
    unsafe { fp(p) }.kind
}
unsafe extern "C" fn p_name(p: *mut c_void) -> *const CStringRepr {
    &unsafe { fp(p) }.name
}
unsafe extern "C" fn p_enabled(p: *mut c_void) -> u8 {
    let p = unsafe { fp(p) };
    if p.stop_on_enabled.get() {
        p.context.set(false);
    }
    u8::from(p.enabled.get())
}
unsafe extern "C" fn p_state(p: *mut c_void, mask: i32) -> u8 {
    u8::from(mask == 0x1000 && unsafe { fp(p) }.readonly.get())
}
unsafe extern "C" fn p_tag(_: *mut c_void) -> i32 {
    106
}
fn raw_float(p: &FakeParameter) -> f64 {
    match &*p.value.borrow() {
        FakeValue::Number(Number::Float(n)) => *n,
        FakeValue::Number(Number::Integer(n)) => *n as f64,
        _ => 0.0,
    }
}
fn number_float(n: Number) -> f64 {
    match n {
        Number::Integer(n) => n as f64,
        Number::Float(n) => n,
    }
}
unsafe extern "C" fn p_norm(p: *mut c_void) -> f32 {
    let p = unsafe { fp(p) };
    if p.kind == 4 && number_float(p.max) == number_float(p.min) {
        return 0.0;
    }
    ((raw_float(p) - number_float(p.min)) / (number_float(p.max) - number_float(p.min))) as f32
}
unsafe extern "C" fn p_set(p: *mut c_void, v: *const Variant, _: u8) {
    let p = unsafe { fp(p) };
    let v = unsafe { &*v };
    p.writes.set(p.writes.get() + 1);
    let n = match v.tag & 15 {
        1 => Number::Integer(v.payload as i64),
        _ => Number::Float(f64::from_bits(v.payload)),
    };
    *p.value.borrow_mut() = FakeValue::Number(n);
    if p.stop_on_set.get() {
        p.context.set(false);
    }
}
fn set_norm(p: &FakeParameter, n: f32) {
    let mut value =
        number_float(p.min) + f64::from(n) * (number_float(p.max) - number_float(p.min));
    let current_integer = matches!(*p.value.borrow(), FakeValue::Number(Number::Integer(_)));
    if p.quantize || current_integer {
        value = value.round();
    }
    *p.value.borrow_mut() = FakeValue::Number(if current_integer {
        Number::Integer(value as i64)
    } else {
        Number::Float(value)
    });
}
unsafe extern "C" fn p_set_norm(p: *mut c_void, n: f32, _: u8) {
    let p = unsafe { fp(p) };
    p.writes.set(p.writes.get() + 1);
    if !p.no_op_normalized.get() {
        set_norm(p, n);
    }
    if p.stop_on_set.get() {
        p.context.set(false);
    }
}
unsafe extern "C" fn p_formatter(p: *mut c_void) -> *mut c_void {
    unsafe { fp(p) }.formatter
}
unsafe extern "C" fn p_display(p: *mut c_void, out: *mut StringRepr) {
    let p = unsafe { fp(p) };
    let text = match &*p.value.borrow() {
        FakeValue::Number(Number::Integer(n)) => n.to_string(),
        FakeValue::Number(Number::Float(n)) => n.to_string(),
        FakeValue::Text(s) => s.clone(),
        FakeValue::Object(_) => "opaque".into(),
        FakeValue::Empty => String::new(),
    };
    let out = unsafe { &mut *out };
    if !out.object.is_null() {
        unsafe {
            string_release(out.object);
        }
    }
    out.object = string_new(&text, p.stats.clone());
    out.private = std::ptr::null_mut();
}
unsafe extern "C" fn p_choice_string(p: *mut c_void, out: *mut StringRepr, value: *const Variant) {
    let p = unsafe { fp(p) };
    let value = unsafe { &*value };
    let index = if value.tag & 15 == 1 {
        value.payload as i64
    } else {
        -999
    };
    p.choice_calls.borrow_mut().push(index);
    let label = p
        .choices
        .borrow()
        .get(index as usize)
        .cloned()
        .unwrap_or_else(|| format!("Choice {index}"));
    let out = unsafe { &mut *out };
    if !out.object.is_null() {
        unsafe {
            string_release(out.object);
        }
    }
    out.object = string_new(&label, p.stats.clone());
    out.private = std::ptr::null_mut();
    let s = unsafe { fs(out.object) };
    s.fail_copy.set(p.choice_copy_fails.get());
    s.skip_termination.set(p.choice_skip_termination.get());
    if p.choice_invalid_utf16.get() {
        *s.units.borrow_mut() = vec![0xd800];
    }
    if p.choice_stop_on_copy.get() {
        *s.stop_on_copy.borrow_mut() = Some(p.context.clone());
    }
    if p.choice_stop_on_drop.get() {
        *s.stop_on_release.borrow_mut() = Some(p.context.clone());
        s.final_release_only.set(true);
    }
    if p.choice_stop_on_get.get() {
        p.context.set(false);
    }
    if let Some(max) = p.choice_max_after_get.get() {
        p.max_override.set(Some(max));
    }
}
fn input_text(text: *const StringRepr) -> String {
    unsafe { String::from_utf16(&fs((*text).object).units.borrow()).unwrap_or_default() }
}
unsafe extern "C" fn p_from_string(p: *mut c_void, text: *const StringRepr, _: u8) {
    let p = unsafe { fp(p) };
    if p.stop_on_input_release.get() {
        *unsafe { fs((*text).object) }.stop_on_release.borrow_mut() = Some(p.context.clone());
    }
    if p.stop_on_formatter_release.get() && !p.formatter.is_null() {
        *unsafe { ff(p.formatter) }.stop_on_release.borrow_mut() = Some(p.context.clone());
        unsafe { ff(p.formatter) }.stop_at_refs.set(Some(3));
    }
    p.writes.set(p.writes.get() + 1);
    let text = input_text(text);
    if p.kind == 3 {
        *p.value.borrow_mut() = FakeValue::Text(text);
    } else {
        let parsed = text
            .trim()
            .trim_end_matches(" dB")
            .parse::<f64>()
            .unwrap_or(0.0);
        if !p.formatter.is_null() && unsafe { ff(p.formatter) }.flags.get() & 1 != 0 {
            set_norm(p, parsed as f32);
        } else {
            let integer = matches!(*p.value.borrow(), FakeValue::Number(Number::Integer(_)));
            *p.value.borrow_mut() = FakeValue::Number(if integer {
                Number::Integer(parsed as i64)
            } else {
                Number::Float(parsed)
            });
        }
    }
    if p.stop_on_set.get() {
        p.context.set(false);
    }
}
#[repr(C)]
struct FakeFormatter {
    table: *const usize,
    refs: Cell<u32>,
    flags: Cell<i32>,
    scripted_flags: RefCell<std::collections::VecDeque<i32>>,
    reset_flags_after_scan: Cell<Option<i32>>,
    stop_on_release: RefCell<Option<Rc<Cell<bool>>>>,
    stop_at_refs: Cell<Option<u32>>,
    scans: Cell<u32>,
    stats: Rc<Stats>,
    wrong_result: bool,
}
unsafe fn ff<'a>(p: *mut c_void) -> &'a FakeFormatter {
    unsafe { &*p.cast() }
}
unsafe extern "C" fn f_retain(p: *mut c_void) -> u32 {
    let f = unsafe { ff(p) };
    let n = f.refs.get() + 1;
    f.refs.set(n);
    n
}
unsafe extern "C" fn f_release(p: *mut c_void) -> u32 {
    let f = unsafe { ff(p) };
    if let (true, Some(context)) = (
        f.stop_at_refs.get().is_none_or(|refs| refs == f.refs.get()),
        f.stop_on_release.borrow().as_ref(),
    ) {
        context.set(false);
    }
    let n = f.refs.get() - 1;
    f.refs.set(n);
    if n == 0 {
        drop(unsafe { Box::from_raw(p.cast::<FakeFormatter>()) });
    }
    n
}
unsafe extern "C" fn f_flags(p: *mut c_void) -> i32 {
    let f = unsafe { ff(p) };
    if let Some(flags) = f.scripted_flags.borrow_mut().pop_front() {
        f.flags.set(flags);
    }
    f.flags.get()
}
unsafe extern "C" fn f_scan(p: *mut c_void, out: *mut Variant, input: *const StringRepr) -> u8 {
    let f = unsafe { ff(p) };
    f.scans.set(f.scans.get() + 1);
    let out = unsafe { &mut *out };
    if f.wrong_result {
        out.tag = 0x13;
        out.payload = string_new("rejected", f.stats.clone()) as usize as u64;
        return 0;
    }
    let text = input_text(input);
    let Ok(value) = text.trim().trim_end_matches(" dB").parse::<f64>() else {
        return 0;
    };
    write_number(out, Number::Float(value));
    if let Some(flags) = f.reset_flags_after_scan.get() {
        f.flags.set(flags);
    }
    1
}
fn formatter_new(flags: i32, stats: Rc<Stats>, wrong_result: bool) -> *mut c_void {
    static VT: OnceLock<[usize; 6]> = OnceLock::new();
    let table = table(&VT, || {
        let mut t = [0; 6];
        t[1] = f_retain as *const () as usize;
        t[2] = f_release as *const () as usize;
        t[3] = f_flags as *const () as usize;
        t[5] = f_scan as *const () as usize;
        t
    });
    Box::into_raw(Box::new(FakeFormatter {
        table,
        refs: Cell::new(1),
        flags: Cell::new(flags),
        scripted_flags: RefCell::new(std::collections::VecDeque::new()),
        reset_flags_after_scan: Cell::new(None),
        stop_on_release: RefCell::new(None),
        stop_at_refs: Cell::new(None),
        scans: Cell::new(0),
        stats,
        wrong_result,
    }))
    .cast()
}
struct Fixture {
    pointer: *mut c_void,
}
impl Fixture {
    fn new(
        env: &TextEnv,
        kind: i32,
        value: FakeValue,
        min: Number,
        max: Number,
        formatter: Option<(i32, bool)>,
    ) -> (Self, RawParameter) {
        static VT: OnceLock<[usize; 46]> = OnceLock::new();
        static OT: OnceLock<[usize; 5]> = OnceLock::new();
        let table = table(&VT, || {
            let mut t = [0; 46];
            t[0] = p_query as *const () as usize;
            t[1] = p_retain as *const () as usize;
            t[2] = p_release as *const () as usize;
            t[3] = p_kind as *const () as usize;
            t[4] = p_name as *const () as usize;
            t[6] = p_enabled as *const () as usize;
            t[8] = p_state as *const () as usize;
            t[13] = p_tag as *const () as usize;
            t[21] = p_set as *const () as usize;
            t[35] = p_norm as *const () as usize;
            t[36] = p_set_norm as *const () as usize;
            t[41] = p_formatter as *const () as usize;
            t[43] = p_choice_string as *const () as usize;
            t[44] = p_display as *const () as usize;
            t[45] = p_from_string as *const () as usize;
            t
        });
        let object_table = table_fn_object(&OT);
        let mut p = Box::new(FakeParameter {
            table,
            refs: Cell::new(2),
            object: ObjectView {
                table: object_table,
                parent: std::ptr::null_mut(),
            },
            name: CStringRepr {
                text: c"outerAliasName".as_ptr(),
                owner: std::ptr::null_mut(),
            },
            kind,
            value: RefCell::new(value),
            min,
            max,
            default: min,
            enabled: Cell::new(true),
            readonly: Cell::new(false),
            writes: Cell::new(0),
            reads: Cell::new(0),
            context: Rc::new(Cell::new(true)),
            stop_on_enabled: Cell::new(false),
            stop_on_set: Cell::new(false),
            no_op_normalized: Cell::new(false),
            stop_on_formatter_release: Cell::new(false),
            stop_on_input_release: Cell::new(false),
            formatter: formatter.map_or(std::ptr::null_mut(), |(flags, wrong)| {
                formatter_new(flags, env.stats.clone(), wrong)
            }),
            quantize: false,
            stats: env.stats.clone(),
            false_property: Cell::new(false),
            choices: RefCell::new(Vec::new()),
            choice_calls: RefCell::new(Vec::new()),
            choice_copy_fails: Cell::new(false),
            choice_skip_termination: Cell::new(false),
            choice_invalid_utf16: Cell::new(false),
            choice_stop_on_get: Cell::new(false),
            choice_stop_on_copy: Cell::new(false),
            choice_stop_on_drop: Cell::new(false),
            choice_max_after_get: Cell::new(None),
            max_override: Cell::new(None),
        });
        p.object.parent = &mut *p;
        let pointer = Box::into_raw(p).cast();
        let raw = unsafe { RawParameter::from_owned(NonNull::new(pointer).unwrap(), text_api()) };
        (Self { pointer }, raw)
    }
    fn p(&self) -> &FakeParameter {
        unsafe { fp(self.pointer) }
    }
    fn guard(&self) -> impl FnMut() -> Result<(), String> {
        let context = self.p().context.clone();
        move || {
            if context.get() {
                Ok(())
            } else {
                Err("context replaced or stopped".into())
            }
        }
    }
}
fn table_fn_object(lock: &OnceLock<[usize; 5]>) -> *const usize {
    table(lock, || {
        let mut t = [0; 5];
        t[1] = o_retain as *const () as usize;
        t[2] = o_release as *const () as usize;
        t[4] = property as *const () as usize;
        t
    })
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            p_release(self.pointer);
        };
    }
}
fn float_fixture(env: &TextEnv) -> (Fixture, RawParameter) {
    Fixture::new(
        env,
        2,
        FakeValue::Number(Number::Float(1.0)),
        Number::Float(0.0),
        Number::Float(2.0),
        None,
    )
}

#[test]
fn describe_reads_actual_outer_identity_and_all_scalar_properties() {
    let env = TextEnv::new();
    let (_f, p) = float_fixture(&env);
    let d = p.describe().unwrap();
    assert_eq!(
        (d.name.as_str(), d.tag, d.kind),
        ("outerAliasName", 106, Kind::Float)
    );
    assert_eq!(d.raw.unwrap().value, Value::Float(1.0));
    assert_eq!(d.default_value.unwrap().value, Value::Float(0.0));
    assert_eq!(d.normalized, Normalized::Finite(0.5));
    assert_eq!(d.display, "1");
    assert!(d.support.raw_number && d.support.display_text);
}
#[test]
fn numeric_range_and_lossless_integer_coercion_do_not_round_boundaries() {
    use value::{coerce, compare, in_range};
    let large = 9_007_199_254_740_993_i64;
    assert_eq!(
        compare(
            Number::Integer(large),
            Number::Float(9_007_199_254_740_992.0)
        ),
        Ordering::Greater
    );
    assert_eq!(
        coerce(Number::Integer(large), Number::Float(0.0)),
        Err(Error::TypeMismatch)
    );
    assert_eq!(
        coerce(Number::Integer(i64::MAX), Number::Float(0.0)),
        Err(Error::TypeMismatch)
    );
    assert_eq!(
        compare(
            Number::Integer(i64::MAX),
            Number::Float(9_223_372_036_854_775_808.0)
        ),
        Ordering::Less
    );
    assert!(in_range(Number::Float(-0.0), Number::Float(0.0), Number::Float(1.0)).is_ok());
}
#[test]
fn raw_change_has_exact_readback_including_negative_zero() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    let result = p
        .set_raw_number(Number::Float(-0.0), &mut f.guard())
        .unwrap();
    assert!(result.verified);
    assert_eq!(f.p().writes.get(), 1);
    assert!(
        matches!(result.after.raw.unwrap().value,Value::Float(n) if n.to_bits()==(-0.0_f64).to_bits())
    );
}
#[test]
fn read_only_disabled_range_nan_and_nonintegral_inputs_never_dispatch() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    f.p().readonly.set(true);
    assert_eq!(
        p.set_raw_number(Number::Float(1.1), &mut f.guard())
            .unwrap_err()
            .error,
        Error::ReadOnly
    );
    f.p().readonly.set(false);
    f.p().enabled.set(false);
    assert_eq!(
        p.set_normalized(0.2, &mut f.guard()).unwrap_err().error,
        Error::Disabled
    );
    f.p().enabled.set(true);
    for n in [f64::NAN, f64::INFINITY, 3.0] {
        assert!(p.set_raw_number(Number::Float(n), &mut f.guard()).is_err());
    }
    assert_eq!(f.p().writes.get(), 0);
    let (i, ip) = Fixture::new(
        &env,
        1,
        FakeValue::Number(Number::Integer(1)),
        Number::Integer(0),
        Number::Integer(10),
        None,
    );
    assert_eq!(
        ip.set_raw_number(Number::Float(1.25), &mut i.guard())
            .unwrap_err()
            .error,
        Error::TypeMismatch
    );
    assert_eq!(i.p().writes.get(), 0);
}
#[test]
fn guards_block_prewrite_and_stop_afterwrite_without_stale_reads() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    f.p().stop_on_enabled.set(true);
    let e = p
        .set_raw_number(Number::Float(1.1), &mut f.guard())
        .unwrap_err();
    assert_eq!(e.outcome, Outcome::NotApplied);
    assert_eq!(f.p().writes.get(), 0);
    f.p().stop_on_enabled.set(false);
    f.p().context.set(true);
    f.p().stop_on_set.set(true);
    let reads = f.p().reads.get();
    let e = p
        .set_raw_number(Number::Float(1.1), &mut f.guard())
        .unwrap_err();
    assert_eq!(e.outcome, Outcome::Unknown);
    assert!(e.after.is_none());
    assert_eq!(f.p().writes.get(), 1);
    assert_eq!(
        f.p().reads.get() - reads,
        8,
        "only two descriptions before setter"
    );
}
#[test]
fn normalized_quantization_reports_observed_mismatch() {
    let env = TextEnv::new();
    let (f, p) = Fixture::new(
        &env,
        1,
        FakeValue::Number(Number::Integer(1)),
        Number::Integer(0),
        Number::Integer(3),
        None,
    );
    let r = p.set_normalized(0.5, &mut f.guard()).unwrap();
    assert!(!r.verified);
    assert_eq!(r.after.raw.unwrap().value, Value::Integer(2));
    assert_eq!(f.p().writes.get(), 1);
    assert!(p.set_normalized(f32::NAN, &mut f.guard()).is_err());
    assert!(p.set_normalized(1.1, &mut f.guard()).is_err());
    assert_eq!(f.p().writes.get(), 1);
}
#[test]
fn formatter_scan_is_outref_bounded_and_stateful_is_not_previewed() {
    let env = TextEnv::new();
    let (f, p) = Fixture::new(
        &env,
        2,
        FakeValue::Number(Number::Float(1.0)),
        Number::Float(0.0),
        Number::Float(2.0),
        Some((0, false)),
    );
    assert!(
        p.set_display_text("1.25 dB", &mut f.guard())
            .unwrap()
            .verified
    );
    assert_eq!(unsafe { ff(f.p().formatter) }.scans.get(), 1);
    assert_eq!(
        p.set_display_text("3 dB", &mut f.guard())
            .unwrap_err()
            .error,
        Error::OutOfRange
    );
    assert_eq!(f.p().writes.get(), 1);
    let (s, sp) = Fixture::new(
        &env,
        2,
        FakeValue::Number(Number::Float(1.0)),
        Number::Float(0.0),
        Number::Float(2.0),
        Some((2, false)),
    );
    assert_eq!(
        sp.set_display_text("1", &mut s.guard()).unwrap_err().error,
        Error::UnsupportedDisplayParser
    );
    assert_eq!(unsafe { ff(s.p().formatter) }.scans.get(), 0);
    assert_eq!(s.p().writes.get(), 0);
}
#[test]
fn normalized_formatter_preview_uses_zero_to_one_and_raw_uses_native_range() {
    let env = TextEnv::new();
    let (f, p) = Fixture::new(
        &env,
        2,
        FakeValue::Number(Number::Float(0.0)),
        Number::Float(-20.0),
        Number::Float(20.0),
        Some((1, false)),
    );
    let r = p.set_display_text("0.75", &mut f.guard()).unwrap();
    assert!(r.verified);
    assert_eq!(r.after.raw.unwrap().value, Value::Float(10.0));
    assert_eq!(
        p.set_display_text("2", &mut f.guard()).unwrap_err().error,
        Error::OutOfRange
    );
    assert_eq!(f.p().writes.get(), 1);
}
#[test]
fn plain_numeric_text_rejects_units_and_lists_only_accept_numeric_indices() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    assert!(p.set_display_text("1.25", &mut f.guard()).unwrap().verified);
    assert_eq!(
        p.set_display_text("1 dB", &mut f.guard())
            .unwrap_err()
            .error,
        Error::InvalidText
    );
    let (l, lp) = Fixture::new(
        &env,
        4,
        FakeValue::Number(Number::Integer(0)),
        Number::Integer(0),
        Number::Integer(3),
        None,
    );
    assert!(
        lp.set_raw_number(Number::Integer(2), &mut l.guard())
            .unwrap()
            .verified
    );
    assert_eq!(
        lp.set_display_text("Two", &mut l.guard())
            .unwrap_err()
            .error,
        Error::UnsupportedDisplayParser
    );
    assert_eq!(l.p().writes.get(), 1);
}
#[test]
fn unicode_string_input_is_cloned_and_owned_output_is_released() {
    let env = TextEnv::new();
    let (f, p) = Fixture::new(
        &env,
        3,
        FakeValue::Text("before".into()),
        Number::Integer(0),
        Number::Integer(0),
        None,
    );
    let r = p.set_display_text("한글 🎸", &mut f.guard()).unwrap();
    assert!(r.verified);
    assert_eq!(r.after.raw.unwrap().value, Value::Text("한글 🎸".into()));
    assert_eq!(
        unsafe { string_len(env.empty) },
        0,
        "singleton never mutated"
    );
    assert_eq!(
        p.set_display_text("bad\0text", &mut f.guard())
            .unwrap_err()
            .error,
        Error::InvalidText
    );
    assert_eq!(f.p().writes.get(), 1);
}
#[test]
fn false_and_rejected_outrefs_release_current_string_or_object_owner() {
    let env = TextEnv::new();
    let (f, p) = Fixture::new(
        &env,
        3,
        FakeValue::Text("discarded".into()),
        Number::Integer(0),
        Number::Integer(0),
        None,
    );
    f.p().false_property.set(true);
    assert!(p.describe().unwrap().raw.is_none());
    let (fmt, fmtp) = Fixture::new(
        &env,
        2,
        FakeValue::Number(Number::Float(1.0)),
        Number::Float(0.0),
        Number::Float(2.0),
        Some((0, true)),
    );
    assert_eq!(
        fmtp.set_display_text("1.1", &mut fmt.guard())
            .unwrap_err()
            .error,
        Error::InvalidText
    );
    assert_eq!(fmt.p().writes.get(), 0);
    let object = string_new("opaque owner", env.stats.clone());
    let (i, ip) = Fixture::new(
        &env,
        7,
        FakeValue::Object(object),
        Number::Integer(0),
        Number::Integer(0),
        None,
    );
    let d = ip.describe().unwrap();
    assert_eq!(d.raw.unwrap().value, Value::OpaqueObject { present: true });
    assert!(!d.support.raw_number && !d.support.normalized && !d.support.display_text);
    assert!(ip.set_normalized(0.2, &mut i.guard()).is_err());
    assert_eq!(i.p().writes.get(), 0);
    unsafe {
        string_release(object);
    };
}
#[test]
fn string_copy_failure_invalid_utf16_and_overflow_never_become_empty_success() {
    let env = TextEnv::new();
    let pointer = string_new("text", env.stats.clone());
    let s = unsafe { fs(pointer) };
    s.fail_copy.set(true);
    assert!(matches!(
        text::copy_i_string(pointer),
        Err(Error::NativeFailure("IString.copyTo", -1))
    ));
    s.fail_copy.set(false);
    *s.units.borrow_mut() = vec![0xd800];
    assert_eq!(text::copy_i_string(pointer), Err(Error::InvalidText));
    *s.units.borrow_mut() = vec![1; MAX_TEXT_UNITS + 1];
    assert_eq!(text::copy_i_string(pointer), Err(Error::TextTooLong));
    unsafe {
        string_release(pointer);
    };
}
#[test]
fn unsupported_command_and_empty_numeric_cannot_dispatch() {
    let env = TextEnv::new();
    let (f, p) = Fixture::new(
        &env,
        5,
        FakeValue::Empty,
        Number::Integer(0),
        Number::Integer(0),
        None,
    );
    assert!(
        p.set_raw_number(Number::Integer(1), &mut f.guard())
            .is_err()
    );
    assert!(p.set_display_text("Run", &mut f.guard()).is_err());
    assert_eq!(f.p().writes.get(), 0);
}

#[test]
fn formatter_release_reentry_after_setter_is_unknown_before_stale_readback() {
    let env = TextEnv::new();
    let (f, p) = Fixture::new(
        &env,
        2,
        FakeValue::Number(Number::Float(1.0)),
        Number::Float(0.0),
        Number::Float(2.0),
        Some((0, false)),
    );
    f.p().stop_on_formatter_release.set(true);
    let post_before_cleanup = Cell::new(false);
    let mut guard = || {
        if f.p().writes.get() == 1 && f.p().context.get() {
            post_before_cleanup.set(true);
        }
        if f.p().context.get() {
            Ok(())
        } else {
            Err("stopped by release".into())
        }
    };
    let result = p.set_display_text("1.25 dB", &mut guard);
    assert!(
        matches!(
            result,
            Err(MutationError {
                outcome: Outcome::Unknown,
                after: None,
                ..
            })
        ),
        "cleanup reentry must precede readback/success: {result:?}"
    );
    assert_eq!(f.p().writes.get(), 1);
    assert!(
        post_before_cleanup.get(),
        "first postsetter guard runs before native cleanup"
    );
    assert_eq!(
        f.p().reads.get(),
        8,
        "no target reads after cleanup invalidates context"
    );
}

#[test]
fn input_string_release_reentry_after_setter_is_unknown_before_stale_readback() {
    let env = TextEnv::new();
    let (f, p) = Fixture::new(
        &env,
        3,
        FakeValue::Text("before".into()),
        Number::Integer(0),
        Number::Integer(0),
        None,
    );
    f.p().stop_on_input_release.set(true);
    let post_before_cleanup = Cell::new(false);
    let mut guard = || {
        if f.p().writes.get() == 1 && f.p().context.get() {
            post_before_cleanup.set(true);
        }
        if f.p().context.get() {
            Ok(())
        } else {
            Err("stopped by release".into())
        }
    };
    let result = p.set_display_text("한글 🎸", &mut guard);
    assert!(
        matches!(
            result,
            Err(MutationError {
                outcome: Outcome::Unknown,
                after: None,
                ..
            })
        ),
        "cleanup reentry must precede readback/success: {result:?}"
    );
    assert_eq!(f.p().writes.get(), 1);
    assert!(
        post_before_cleanup.get(),
        "first postsetter guard runs before native cleanup"
    );
    assert_eq!(f.p().reads.get(), 8);
}

#[test]
fn transient_scan_flags_cannot_be_hidden_by_matching_initial_and_final_flags() {
    let env = TextEnv::new();
    let (f, p) = Fixture::new(
        &env,
        2,
        FakeValue::Number(Number::Float(1.0)),
        Number::Float(0.0),
        Number::Float(2.0),
        Some((0, false)),
    );
    let formatter = unsafe { ff(f.p().formatter) };
    *formatter.scripted_flags.borrow_mut() = [0, 1].into();
    formatter.reset_flags_after_scan.set(Some(0));
    let result = p.set_display_text("0.75", &mut f.guard());
    assert!(
        matches!(
            result,
            Err(MutationError {
                outcome: Outcome::NotApplied,
                error: Error::MetadataChanged,
                ..
            })
        ),
        "scan flags must match final flags: {result:?}"
    );
    assert_eq!(f.p().writes.get(), 0);
}

fn list_fixture(env: &TextEnv, current: i64, max: i64) -> (Fixture, RawParameter) {
    Fixture::new(
        env,
        4,
        FakeValue::Number(Number::Integer(current)),
        Number::Integer(0),
        Number::Integer(max),
        None,
    )
}
#[test]
fn list_choice_pages_read_unicode_labels_without_selecting_any_index() {
    let env = TextEnv::new();
    let (f, p) = list_fixture(&env, -1, 2);
    *f.p().choices.borrow_mut() = vec!["Mono".into(), "스테레오 🎸".into(), "".into()];
    let first = p.list_choices(0, 2, &mut f.guard()).unwrap();
    assert_eq!((first.count, first.offset, first.next), (3, 0, Some(2)));
    assert_eq!(
        first.choices,
        vec![
            ListChoice {
                index: 0,
                label: "Mono".into()
            },
            ListChoice {
                index: 1,
                label: "스테레오 🎸".into()
            }
        ]
    );
    let last = p.list_choices(2, 128, &mut f.guard()).unwrap();
    assert_eq!(last.next, None);
    assert_eq!(last.choices[0].label, "");
    assert_eq!(*f.p().choice_calls.borrow(), vec![0, 1, 2]);
    assert_eq!(f.p().writes.get(), 0);
    assert_eq!(
        value::number(p.describe().unwrap().raw.as_ref()),
        Some(Number::Integer(-1))
    );
}
#[test]
fn empty_list_choices_have_zero_count_and_remain_nonwritable() {
    let env = TextEnv::new();
    let (f, p) = list_fixture(&env, -1, -1);
    let page = p.list_choices(0, 128, &mut f.guard()).unwrap();
    assert_eq!(
        page,
        ListChoices {
            count: 0,
            offset: 0,
            next: None,
            choices: vec![]
        }
    );
    assert!(f.p().choice_calls.borrow().is_empty());
    let d = p.describe().unwrap();
    assert!(!d.support.raw_number && !d.support.normalized && !d.support.display_text);
    assert!(
        p.set_raw_number(Number::Integer(0), &mut f.guard())
            .is_err()
    );
    assert!(p.set_normalized(0.0, &mut f.guard()).is_err());
    assert_eq!(f.p().writes.get(), 0);
}
#[test]
fn choice_reads_allow_readonly_disabled_lists_but_reject_other_types_and_bounds() {
    let env = TextEnv::new();
    let (f, p) = list_fixture(&env, 0, 1);
    f.p().enabled.set(false);
    f.p().readonly.set(true);
    assert_eq!(
        p.list_choices(0, 1, &mut f.guard()).unwrap().choices.len(),
        1
    );
    assert!(p.list_choices(0, 0, &mut f.guard()).is_err());
    assert!(p.list_choices(0, 129, &mut f.guard()).is_err());
    assert_eq!(p.list_choices(3, 1, &mut f.guard()), Err(Error::OutOfRange));
    let (float, fp) = float_fixture(&env);
    assert_eq!(
        fp.list_choices(0, 1, &mut float.guard()),
        Err(Error::UnsupportedKind)
    );
    let (huge, hp) = list_fixture(&env, 0, i64::MAX);
    assert!(hp.list_choices(0, 1, &mut huge.guard()).is_err());
    assert!(huge.p().choice_calls.borrow().is_empty());
    let (bad, bp) = list_fixture(&env, -1, -2);
    assert!(bp.list_choices(0, 1, &mut bad.guard()).is_err());
    assert!(bad.p().choice_calls.borrow().is_empty());
    let (fraction, fractp) = Fixture::new(
        &env,
        4,
        FakeValue::Number(Number::Integer(0)),
        Number::Integer(0),
        Number::Float(1.5),
        None,
    );
    assert!(fractp.list_choices(0, 1, &mut fraction.guard()).is_err());
    assert!(fraction.p().choice_calls.borrow().is_empty());
}
#[test]
fn choice_copy_errors_and_invalid_utf16_or_missing_termination_discard_page() {
    let env = TextEnv::new();
    let (f, p) = list_fixture(&env, 0, 1);
    f.p().choice_copy_fails.set(true);
    assert!(matches!(
        p.list_choices(0, 2, &mut f.guard()),
        Err(Error::NativeFailure("IString.copyTo", -1))
    ));
    assert_eq!(*f.p().choice_calls.borrow(), vec![0]);
    f.p().choice_copy_fails.set(false);
    f.p().choice_invalid_utf16.set(true);
    assert_eq!(
        p.list_choices(0, 1, &mut f.guard()),
        Err(Error::InvalidText)
    );
    f.p().choice_invalid_utf16.set(false);
    f.p().choice_skip_termination.set(true);
    assert!(matches!(
        p.list_choices(0, 1, &mut f.guard()),
        Err(Error::InvalidData(_))
    ));
    assert_eq!(f.p().writes.get(), 0);
}
#[test]
fn stale_choice_guards_cover_get_copy_and_final_output_owner_cleanup() {
    let env = TextEnv::new();
    for hook in 0..3 {
        let (f, p) = list_fixture(&env, -1, 2);
        match hook {
            0 => f.p().choice_stop_on_get.set(true),
            1 => f.p().choice_stop_on_copy.set(true),
            _ => f.p().choice_stop_on_drop.set(true),
        }
        assert!(matches!(
            p.list_choices(0, 3, &mut f.guard()),
            Err(Error::Guard(_))
        ));
        assert_eq!(*f.p().choice_calls.borrow(), vec![0]);
        assert_eq!(f.p().writes.get(), 0);
    }
}
#[test]
fn choice_range_changes_and_initial_stale_context_never_publish_labels() {
    let env = TextEnv::new();
    let (f, p) = list_fixture(&env, -1, 1);
    f.p().choice_max_after_get.set(Some(Number::Integer(2)));
    assert_eq!(
        p.list_choices(0, 1, &mut f.guard()),
        Err(Error::MetadataChanged)
    );
    assert_eq!(f.p().writes.get(), 0);
    // Once a range change has been observed, later original-page indices must
    // not be sent to the native label getter at all.
    f.p().max_override.set(None);
    f.p().choice_calls.borrow_mut().clear();
    f.p().choice_max_after_get.set(Some(Number::Integer(0)));
    assert_eq!(
        p.list_choices(0, 2, &mut f.guard()),
        Err(Error::MetadataChanged)
    );
    assert_eq!(*f.p().choice_calls.borrow(), vec![0]);
    f.p().context.set(false);
    f.p().choice_calls.borrow_mut().clear();
    assert!(matches!(
        p.list_choices(0, 1, &mut f.guard()),
        Err(Error::Guard(_))
    ));
    assert!(f.p().choice_calls.borrow().is_empty());
}
#[test]
fn only_unselected_integer_list_can_select_a_valid_new_index() {
    let env = TextEnv::new();
    let (f, p) = list_fixture(&env, -1, 2);
    assert!(p.describe().unwrap().support.raw_number);
    assert!(
        p.set_raw_number(Number::Integer(-1), &mut f.guard())
            .is_err()
    );
    assert_eq!(f.p().writes.get(), 0);
    assert!(
        p.set_raw_number(Number::Integer(1), &mut f.guard())
            .unwrap()
            .verified
    );
    let (normalized_f, normalized_p) = list_fixture(&env, -1, 2);
    assert!(
        normalized_p
            .set_normalized(0.0, &mut normalized_f.guard())
            .unwrap()
            .verified
    );
    assert_eq!(
        normalized_p.describe().unwrap().raw.unwrap().value,
        Value::Integer(0)
    );
    for kind in [0, 1, 2, 9] {
        let (f, p) = Fixture::new(
            &env,
            kind,
            FakeValue::Number(Number::Integer(-1)),
            Number::Integer(0),
            Number::Integer(2),
            None,
        );
        assert!(
            p.set_raw_number(Number::Integer(1), &mut f.guard())
                .is_err()
        );
        assert_eq!(f.p().writes.get(), 0);
    }
    let (f, p) = list_fixture(&env, -2, 2);
    assert!(
        p.set_raw_number(Number::Integer(1), &mut f.guard())
            .is_err()
    );
    assert_eq!(f.p().writes.get(), 0);
    let (f, p) = Fixture::new(
        &env,
        4,
        FakeValue::Number(Number::Float(-1.0)),
        Number::Integer(0),
        Number::Integer(2),
        None,
    );
    assert!(
        p.set_raw_number(Number::Integer(1), &mut f.guard())
            .is_err()
    );
    assert_eq!(f.p().writes.get(), 0);
    let (f, p) = Fixture::new(
        &env,
        4,
        FakeValue::Number(Number::Integer(-1)),
        Number::Integer(1),
        Number::Integer(2),
        None,
    );
    assert!(
        p.set_raw_number(Number::Integer(1), &mut f.guard())
            .is_err()
    );
    assert_eq!(f.p().writes.get(), 0);
}

#[test]
fn unselected_single_choice_noop_normalized_write_cannot_be_verified() {
    let env = TextEnv::new();
    let (f, p) = list_fixture(&env, -1, 0);
    assert!(p.describe().unwrap().support.normalized);
    f.p().no_op_normalized.set(true);
    let result = p.set_normalized(0.0, &mut f.guard());
    assert!(
        matches!(
            result,
            Err(MutationError {
                outcome: Outcome::Unknown,
                error: Error::OutOfRange,
                ..
            })
        ),
        "unselected is only a pre-write exception, never a verified selection: {result:?}"
    );
    assert_eq!(f.p().writes.get(), 1);

    let (valid_f, valid) = list_fixture(&env, -1, 0);
    let result = valid.set_normalized(0.0, &mut valid_f.guard()).unwrap();
    assert!(result.verified);
    assert_eq!(result.after.raw.unwrap().value, Value::Integer(0));
}
