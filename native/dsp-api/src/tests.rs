//! Local fake interfaces only. No DAW, loader, application or SDK I/O.
use super::*;
use crate::{
    ffi::{PlainCString, PlainString, Uid},
    record::ParamInfo,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::OnceLock,
};

#[derive(Default)]
struct Stats {
    created: Cell<u32>,
    destroyed: Cell<u32>,
}
#[repr(C)]
struct Owner {
    table: *const usize,
    refs: Cell<u32>,
    bytes: Vec<u8>,
    units: Vec<u16>,
    stats: Rc<Stats>,
    status: i32,
    length_override: Option<i32>,
    change_length: bool,
    no_terminator: bool,
    copied: Cell<bool>,
}
fn owner(bytes: Vec<u8>, units: Vec<u16>, stats: Rc<Stats>) -> *mut c_void {
    static TABLE: OnceLock<[usize; 9]> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut t = [0; 9];
        t[1] = o_retain as *const () as usize;
        t[2] = o_release as *const () as usize;
        t[4] = o_length as *const () as usize;
        t[8] = o_copy as *const () as usize;
        t
    });
    stats.created.set(stats.created.get() + 1);
    Box::into_raw(Box::new(Owner {
        table: table.as_ptr(),
        refs: Cell::new(1),
        bytes,
        units,
        stats,
        status: 0,
        length_override: None,
        change_length: false,
        no_terminator: false,
        copied: Cell::new(false),
    }))
    .cast()
}
unsafe fn own<'a>(p: *mut c_void) -> &'a Owner {
    unsafe { &*p.cast() }
}
unsafe extern "C" fn o_retain(p: *mut c_void) -> u32 {
    let o = unsafe { own(p) };
    let n = o.refs.get() + 1;
    o.refs.set(n);
    n
}
unsafe extern "C" fn o_release(p: *mut c_void) -> u32 {
    let o = unsafe { own(p) };
    let n = o.refs.get() - 1;
    o.refs.set(n);
    if n == 0 {
        o.stats.destroyed.set(o.stats.destroyed.get() + 1);
        drop(unsafe { Box::from_raw(p.cast::<Owner>()) });
    }
    n
}
unsafe extern "C" fn o_length(p: *mut c_void) -> i32 {
    let o = unsafe { own(p) };
    o.length_override.unwrap_or(o.units.len() as i32) + i32::from(o.change_length && o.copied.get())
}
unsafe extern "C" fn o_copy(p: *mut c_void, out: *mut u16, size: i32) -> i32 {
    let o = unsafe { own(p) };
    o.copied.set(true);
    if o.status != 0 {
        return o.status;
    }
    if size < o.units.len() as i32 + 1 {
        return -9;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(o.units.as_ptr(), out, o.units.len());
        if !o.no_terminator {
            *out.add(o.units.len()) = 0;
        }
    }
    0
}
struct Constants {
    string: PlainString,
    cstring: PlainCString,
    null_string: Cell<bool>,
    null_cstring: Cell<bool>,
}
thread_local! {static CONSTANTS:Cell<*const Constants>=const{Cell::new(std::ptr::null())};}
unsafe extern "C" fn constant_string(_: *const c_char) -> *const c_void {
    CONSTANTS.with(|c| {
        let c = unsafe { &*c.get() };
        if c.null_string.get() {
            std::ptr::null()
        } else {
            (&c.string as *const PlainString).cast()
        }
    })
}
unsafe extern "C" fn constant_cstring(_: *const c_char) -> *const c_void {
    CONSTANTS.with(|c| {
        let c = unsafe { &*c.get() };
        if c.null_cstring.get() {
            std::ptr::null()
        } else {
            (&c.cstring as *const PlainCString).cast()
        }
    })
}
struct Env {
    constants: Box<Constants>,
    string: *mut c_void,
    cstring: *mut c_void,
    stats: Rc<Stats>,
}
impl Env {
    fn new() -> Self {
        let stats = Rc::new(Stats::default());
        let string = owner(vec![0], vec![], stats.clone());
        let cstring = owner(vec![0], vec![], stats.clone());
        let constants = Box::new(Constants {
            string: PlainString {
                object: string,
                private: std::ptr::dangling_mut(),
            },
            cstring: PlainCString {
                text: unsafe { own(cstring) }.bytes.as_ptr().cast(),
                owner: cstring,
            },
            null_string: Cell::new(false),
            null_cstring: Cell::new(false),
        });
        CONSTANTS.with(|c| {
            assert!(c.get().is_null());
            c.set(&*constants);
        });
        Self {
            constants,
            string,
            cstring,
            stats,
        }
    }
    fn api(&self) -> TextApi {
        TextApi {
            get_constant_string: constant_string,
            get_constant_cstring: constant_cstring,
        }
    }
    fn balanced(&self) {
        assert_eq!(unsafe { own(self.string) }.refs.get(), 1);
        assert_eq!(unsafe { own(self.cstring) }.refs.get(), 1);
        assert_eq!(self.stats.created.get() - self.stats.destroyed.get(), 2);
    }
}
impl Drop for Env {
    fn drop(&mut self) {
        CONSTANTS.with(|c| c.set(std::ptr::null()));
        unsafe {
            o_release(self.string);
            o_release(self.cstring);
        }
        assert_eq!(
            self.stats.created.get(),
            self.stats.destroyed.get(),
            "all six native output owner paths must balance"
        );
    }
}

#[derive(Clone)]
struct Entry {
    status: u8,
    name: Vec<u8>,
    title: Vec<u16>,
    null_string: bool,
    copy_status: i32,
    length_override: Option<i32>,
    change_length: bool,
    no_terminator: bool,
}
impl Entry {
    fn valid(index: u32) -> Self {
        Self {
            status: 1,
            name: format!("parameter{index}\0").into_bytes(),
            title: format!("파라미터 {index} 🎸").encode_utf16().collect(),
            null_string: false,
            copy_status: 0,
            length_override: None,
            change_length: false,
            no_terminator: false,
        }
    }
}
#[repr(C)]
struct WireInfo {
    tag: u32,
    folder: u32,
    name: PlainCString,
    cstring24: PlainCString,
    title: PlainString,
    string56: PlainString,
    string72: PlainString,
    cstring88: PlainCString,
    float104: f32,
    integer108: u32,
    flags: u32,
    integer116: u32,
    integer120: u64,
}
const _: () = assert!(std::mem::size_of::<WireInfo>() == 128);
#[repr(C)]
struct Provider {
    table: *const usize,
    refs: Cell<u32>,
    entries: Vec<Entry>,
    reported: Cell<i32>,
    calls: RefCell<Vec<i32>>,
    count_calls: Cell<u32>,
    change_count: Cell<Option<i32>>,
    query_result: Cell<i32>,
    null_provider: Cell<bool>,
    iid_match: Cell<bool>,
    stats: Rc<Stats>,
}
unsafe fn prov<'a>(p: *mut c_void) -> &'a Provider {
    unsafe { &*p.cast() }
}
unsafe extern "C" fn p_retain(p: *mut c_void) -> u32 {
    let p = unsafe { prov(p) };
    let n = p.refs.get() + 1;
    p.refs.set(n);
    n
}
unsafe extern "C" fn p_release(p: *mut c_void) -> u32 {
    let p = unsafe { prov(p) };
    let n = p.refs.get() - 1;
    p.refs.set(n);
    if n == 0 {
        drop(unsafe { Box::from_raw((p as *const Provider).cast_mut()) });
    }
    n
}
unsafe extern "C" fn query(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let provider = unsafe { prov(p) };
    let iid = unsafe { &*iid };
    provider.iid_match.set(
        iid.a == 0xd889caea
            && iid.b == 0x8356
            && iid.c == 0x45bb
            && iid.d == [0x8b, 0x45, 0xee, 0xcd, 0x11, 0x31, 0xa9, 0x4d],
    );
    if provider.query_result.get() != 0 {
        return provider.query_result.get();
    }
    if !provider.null_provider.get() {
        unsafe {
            p_retain(p);
            *out = p;
        }
    }
    0
}
unsafe extern "C" fn count(p: *mut c_void) -> i32 {
    let p = unsafe { prov(p) };
    p.count_calls.set(p.count_calls.get() + 1);
    p.reported.get()
}
unsafe extern "C" fn info(p: *mut c_void, out: *mut ParamInfo, index: i32) -> u8 {
    let p = unsafe { prov(p) };
    p.calls.borrow_mut().push(index);
    if let Some(after) = p.change_count.get() {
        p.reported.set(after);
    }
    let Some(entry) = p.entries.get(index as usize) else {
        return 0;
    };
    let out = unsafe { &mut *out.cast::<WireInfo>() };
    let s = owner(vec![0], entry.title.clone(), p.stats.clone());
    let c = owner(entry.name.clone(), vec![], p.stats.clone());
    let sm = unsafe { &mut *s.cast::<Owner>() };
    sm.status = entry.copy_status;
    sm.length_override = entry.length_override;
    sm.change_length = entry.change_length;
    sm.no_terminator = entry.no_terminator;
    for field in [&mut out.title, &mut out.string56, &mut out.string72] {
        unsafe {
            o_retain(s);
            o_release(field.object);
        }
        *field = PlainString {
            object: s,
            private: std::ptr::dangling_mut(),
        };
    }
    for field in [&mut out.name, &mut out.cstring24, &mut out.cstring88] {
        unsafe {
            o_retain(c);
            if !field.owner.is_null() {
                o_release(field.owner);
            }
        }
        *field = PlainCString {
            text: unsafe { own(c) }.bytes.as_ptr().cast(),
            owner: c,
        };
    }
    unsafe {
        o_release(s);
        o_release(c);
    }
    if entry.null_string {
        unsafe {
            o_release(out.string56.object);
        }
        out.string56.object = std::ptr::null_mut();
    }
    out.tag = index as u32 + 100;
    out.folder = 5;
    out.flags = 0x1000_1234;
    entry.status
}
struct Fixture {
    pointer: *mut c_void,
}
impl Fixture {
    fn new(env: &Env, entries: Vec<Entry>) -> Self {
        static TABLE: OnceLock<[usize; 5]> = OnceLock::new();
        let table = TABLE.get_or_init(|| {
            [
                query as *const () as usize,
                p_retain as *const () as usize,
                p_release as *const () as usize,
                count as *const () as usize,
                info as *const () as usize,
            ]
        });
        let pointer = Box::into_raw(Box::new(Provider {
            table: table.as_ptr(),
            refs: Cell::new(1),
            reported: Cell::new(entries.len() as i32),
            entries,
            calls: RefCell::new(Vec::new()),
            count_calls: Cell::new(0),
            change_count: Cell::new(None),
            query_result: Cell::new(0),
            null_provider: Cell::new(false),
            iid_match: Cell::new(false),
            stats: env.stats.clone(),
        }))
        .cast();
        Self { pointer }
    }
    fn native(&self) -> &Provider {
        unsafe { prov(self.pointer) }
    }
    fn wrap(&self, env: &Env) -> Result<DspParameters, Error> {
        unsafe {
            p_retain(self.pointer);
            DspParameters::from_owned_target(NonNull::new(self.pointer).unwrap(), env.api())
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            p_release(self.pointer);
        }
    }
}

#[test]
fn pagination_is_bounded_and_never_claims_unobserved_global_completeness() {
    let env = Env::new();
    let f = Fixture::new(&env, (0..199).map(Entry::valid).collect());
    let p = f.wrap(&env).unwrap();
    assert!(f.native().iid_match.get());
    assert_eq!(p.count(), Ok(199));
    let first = p.page(0, 128).unwrap();
    assert_eq!(
        (first.records.len(), first.attempted, first.next, first.cap),
        (128, 128, Some(128), 128)
    );
    assert!(!first.complete);
    let last = p.page(128, 128).unwrap();
    assert_eq!(
        (last.records.len(), last.next, last.complete),
        (71, None, false)
    );
    assert_eq!(f.native().calls.borrow().len(), 199);
    let record = &last.records[70];
    assert_eq!(
        (record.index, record.tag, record.folder, record.raw_flags),
        (198, 298, 5, 0x1000_1234)
    );
    assert_eq!(record.name, "parameter198");
    assert_eq!(record.title, "파라미터 198 🎸");
    env.balanced();
}
#[test]
fn single_page_and_empty_provider_can_report_complete() {
    let env = Env::new();
    let f = Fixture::new(&env, vec![Entry::valid(0)]);
    let p = f.wrap(&env).unwrap();
    let page = p.page(0, 128).unwrap();
    assert!(page.complete);
    assert!(page.failed_indices.is_empty());
    let empty = Fixture::new(&env, vec![]);
    let ep = empty.wrap(&env).unwrap();
    env.constants.null_string.set(true);
    let page = ep.page(0, 1).unwrap();
    assert!(page.complete);
    assert_eq!(page.attempted, 0);
    assert!(empty.native().calls.borrow().is_empty());
    env.balanced();
}
#[test]
fn false_unexpected_status_and_invalid_owner_continue_with_balanced_cleanup() {
    let env = Env::new();
    let mut entries = (0..4).map(Entry::valid).collect::<Vec<_>>();
    entries[0].status = 0;
    entries[1].status = 2;
    entries[2].null_string = true;
    let f = Fixture::new(&env, entries);
    let p = f.wrap(&env).unwrap();
    let page = p.page(0, 4).unwrap();
    assert_eq!(page.records.len(), 1);
    assert!(!page.complete);
    assert_eq!(*f.native().calls.borrow(), vec![0, 1, 2, 3]);
    assert_eq!(
        page.failed_indices,
        vec![
            FailedIndex {
                index: 0,
                reason: InfoError::NativeStatus(0)
            },
            FailedIndex {
                index: 1,
                reason: InfoError::NativeStatus(2)
            },
            FailedIndex {
                index: 2,
                reason: InfoError::InvalidStringOwners
            }
        ]
    );
    env.balanced();
}
#[test]
fn title_copy_status_termination_length_and_utf16_are_checked() {
    let env = Env::new();
    let mut entries = (0..7).map(Entry::valid).collect::<Vec<_>>();
    entries[0].copy_status = -1;
    entries[1].title = vec![1; 513];
    entries[2].length_override = Some(-1);
    entries[3].no_terminator = true;
    entries[4].change_length = true;
    entries[5].title = vec![0xd800];
    entries[6].title = vec![97; 512];
    let f = Fixture::new(&env, entries);
    let p = f.wrap(&env).unwrap();
    let page = p.page(0, 7).unwrap();
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.records[0].title.len(), 512);
    assert_eq!(
        page.failed_indices
            .iter()
            .map(|f| f.reason.clone())
            .collect::<Vec<_>>(),
        vec![
            InfoError::TitleCopyFailed(-1),
            InfoError::TitleTooLong,
            InfoError::TitleTooLong,
            InfoError::TitleNotTerminated,
            InfoError::TitleChangedLength,
            InfoError::InvalidTitleUtf16
        ]
    );
    env.balanced();
}
#[test]
fn names_are_bounded_valid_utf8_and_can_be_empty() {
    let env = Env::new();
    let mut entries = (0..4).map(Entry::valid).collect::<Vec<_>>();
    entries[0].name = vec![0xff, 0];
    entries[1].name = vec![97; 513];
    entries[1].name.push(0);
    entries[2].name = vec![0];
    entries[3].name = vec![97; 512];
    entries[3].name.push(0);
    let f = Fixture::new(&env, entries);
    let p = f.wrap(&env).unwrap();
    let page = p.page(0, 4).unwrap();
    assert_eq!(page.records.len(), 2);
    assert_eq!(page.records[0].name, "");
    assert_eq!(page.records[1].name.len(), 512);
    assert_eq!(page.failed_indices[0].reason, InfoError::InvalidNameUtf8);
    assert_eq!(page.failed_indices[1].reason, InfoError::NameTooLong);
    env.balanced();
}
#[test]
fn invalid_limits_offsets_and_counts_do_not_dispatch_info() {
    let env = Env::new();
    let f = Fixture::new(&env, vec![Entry::valid(0)]);
    let p = f.wrap(&env).unwrap();
    assert_eq!(p.page(0, 0), Err(Error::InvalidLimit(0)));
    assert_eq!(p.page(0, 129), Err(Error::InvalidLimit(129)));
    assert_eq!(f.native().count_calls.get(), 0);
    assert_eq!(
        p.page(u32::MAX, 1),
        Err(Error::InvalidOffset {
            offset: u32::MAX,
            count: 1
        })
    );
    for n in [-1, 1_000_001, i32::MAX] {
        f.native().reported.set(n);
        assert_eq!(p.count(), Err(Error::InvalidCount(n)));
    }
    assert!(f.native().calls.borrow().is_empty());
    env.balanced();
}
#[test]
fn provider_count_change_discards_page_and_releases_all_outputs() {
    let env = Env::new();
    let f = Fixture::new(&env, vec![Entry::valid(0)]);
    let p = f.wrap(&env).unwrap();
    f.native().change_count.set(Some(2));
    assert_eq!(
        p.page(0, 1),
        Err(Error::CountChanged {
            before: 1,
            after: 2
        })
    );
    env.balanced();
}
#[test]
fn constructor_consumes_target_reference_on_missing_or_null_provider() {
    let env = Env::new();
    let f = Fixture::new(&env, vec![]);
    f.native().query_result.set(-42);
    assert!(matches!(f.wrap(&env), Err(Error::MissingProvider(-42))));
    assert_eq!(f.native().refs.get(), 1);
    f.native().query_result.set(0);
    f.native().null_provider.set(true);
    assert!(matches!(f.wrap(&env), Err(Error::NullProvider)));
    assert_eq!(f.native().refs.get(), 1);
    f.native().null_provider.set(false);
    let p = f.wrap(&env).unwrap();
    assert_eq!(f.native().refs.get(), 3);
    drop(p);
    assert_eq!(f.native().refs.get(), 1);
    env.balanced();
}
#[test]
fn missing_or_invalid_constants_fail_without_info_calls() {
    let mut env = Env::new();
    let f = Fixture::new(&env, vec![Entry::valid(0)]);
    let p = f.wrap(&env).unwrap();
    env.constants.null_string.set(true);
    assert_eq!(p.page(0, 1), Err(Error::MissingTextConstant("String")));
    env.constants.null_string.set(false);
    env.constants.null_cstring.set(true);
    assert_eq!(p.page(0, 1), Err(Error::MissingTextConstant("CString")));
    env.constants.null_cstring.set(false);
    env.constants.string.object = std::ptr::null_mut();
    assert_eq!(p.page(0, 1), Err(Error::InvalidEmptyString));
    assert!(f.native().calls.borrow().is_empty());
    env.balanced();
}
#[test]
fn thread_check_precedes_native_dispatch() {
    let env = Env::new();
    let f = Fixture::new(&env, vec![]);
    let mut p = f.wrap(&env).unwrap();
    p.thread = std::thread::spawn(|| std::thread::current().id())
        .join()
        .unwrap();
    assert_eq!(p.count(), Err(Error::WrongThread));
    assert_eq!(p.page(0, 1), Err(Error::WrongThread));
    assert_eq!(f.native().count_calls.get(), 0);
    env.balanced();
}

#[test]
fn null_cstring_owners_are_valid_and_large_reported_count_stays_bounded() {
    let mut env = Env::new();
    env.constants.cstring.owner = std::ptr::null_mut();
    let successful = Fixture::new(&env, vec![Entry::valid(0)]);
    let p = successful.wrap(&env).unwrap();
    assert!(p.page(0, 1).unwrap().complete);
    env.balanced();

    // A valid count does not allocate count-sized storage or trigger count
    // info calls. Native false leaves the original valid empty fields intact.
    let f = Fixture::new(&env, vec![]);
    f.native().reported.set(MAX_PARAMETER_COUNT as i32);
    let p = f.wrap(&env).unwrap();
    let page = p.page(0, MAX_PAGE_SIZE).unwrap();
    assert_eq!(page.attempted, MAX_PAGE_SIZE);
    assert_eq!(page.failed_indices.len(), MAX_PAGE_SIZE as usize);
    assert_eq!(page.next, Some(MAX_PAGE_SIZE));
    assert_eq!(f.native().calls.borrow().len(), MAX_PAGE_SIZE as usize);
    assert!(!page.complete);
    env.balanced();
}
