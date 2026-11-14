use super::super::*;
use std::sync::Arc;

const STREAM: Uid = Uid {
    a: 0x7fcab9b0,
    b: 0xe595,
    c: 0x4a01,
    d: [0x9e, 0xf1, 0xa7, 0x3f, 0x22, 0xba, 0x89, 0xbd],
};

#[repr(C)]
struct StreamVtable {
    unknown: UnknownVtable,
    read: unsafe extern "C" fn(*mut c_void, *mut c_void, i32) -> i32,
    write: unsafe extern "C" fn(*mut c_void, *const c_void, i32) -> i32,
    tell: unsafe extern "C" fn(*mut c_void) -> i64,
    seekable: unsafe extern "C" fn(*mut c_void) -> u8,
    seek: unsafe extern "C" fn(*mut c_void, i64, i32) -> i64,
}
enum Data {
    Capture(Vec<u8>),
    Restore(Arc<[u8]>),
    Closed,
}
struct State {
    data: Data,
    position: usize,
    limit: usize,
    failed: bool,
}
impl State {
    fn bytes(&self) -> &[u8] {
        match &self.data {
            Data::Capture(bytes) => bytes,
            Data::Restore(bytes) => bytes,
            Data::Closed => &[],
        }
    }
    fn fail<T>(&mut self, value: T) -> T {
        self.failed = true;
        value
    }
}
#[repr(C)]
struct Stream {
    vtable: *const StreamVtable,
    refs: AtomicU32,
    state: Mutex<State>,
}
static VTABLE: StreamVtable = StreamVtable {
    unknown: UnknownVtable {
        query,
        retain,
        release,
    },
    read,
    write,
    tell,
    seekable,
    seek,
};
const _: () = {
    assert!(size_of::<StreamVtable>() == 64);
    assert!(offset_of!(StreamVtable, read) == 24 && offset_of!(StreamVtable, write) == 32);
    assert!(offset_of!(StreamVtable, tell) == 40 && offset_of!(StreamVtable, seekable) == 48);
    assert!(offset_of!(StreamVtable, seek) == 56);
};

/// A native host may retain the stream. Its reference-counted allocation is
/// independent of request/native owners, and is sealed before this owner drops.
pub(super) struct MemoryStream(*mut Stream);
impl MemoryStream {
    pub(super) fn capture(limit: usize) -> Self {
        Self::new(Data::Capture(Vec::new()), limit)
    }
    pub(super) fn restore(bytes: Arc<[u8]>) -> Self {
        let limit = bytes.len();
        Self::new(Data::Restore(bytes), limit)
    }
    fn new(data: Data, limit: usize) -> Self {
        Self(Box::into_raw(Box::new(Stream {
            vtable: &VTABLE,
            refs: AtomicU32::new(1),
            state: Mutex::new(State {
                data,
                position: 0,
                limit,
                failed: false,
            }),
        })))
    }
    pub(super) fn raw(&self) -> *mut c_void {
        self.0.cast()
    }
    pub(super) fn finish_capture(&self) -> Result<Vec<u8>, String> {
        let mut state = unsafe { &*self.0 }
            .state
            .lock()
            .map_err(|_| "stream lock poisoned")?;
        let bytes = std::mem::replace(&mut state.data, Data::Closed);
        if state.failed {
            return Err("native state stream failed or exceeded its bounds".into());
        }
        match bytes {
            Data::Capture(bytes) if !bytes.is_empty() => Ok(bytes),
            _ => Err("native save produced no independent state bytes".into()),
        }
    }
    pub(super) fn finish_restore(&self) -> Result<(), String> {
        let mut state = unsafe { &*self.0 }
            .state
            .lock()
            .map_err(|_| "stream lock poisoned")?;
        let was_reader = matches!(state.data, Data::Restore(_));
        state.data = Data::Closed;
        if state.failed || !was_reader {
            Err("native state input stream failed".into())
        } else {
            Ok(())
        }
    }
}
impl Drop for MemoryStream {
    fn drop(&mut self) {
        if let Ok(mut state) = unsafe { &*self.0 }.state.lock() {
            state.data = Data::Closed;
        }
        unsafe { release(self.raw()) };
    }
}
unsafe extern "C" fn query(this: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    if out.is_null() {
        return INVALID_POINTER;
    }
    unsafe { *out = ptr::null_mut() };
    if iid.is_null() || unsafe { *iid != UNKNOWN && *iid != STREAM } {
        return NO_INTERFACE;
    }
    unsafe {
        retain(this);
        *out = this;
    }
    OK
}
unsafe extern "C" fn retain(this: *mut c_void) -> u32 {
    unsafe { &*this.cast::<Stream>() }
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
unsafe extern "C" fn release(this: *mut c_void) -> u32 {
    let stream = unsafe { &*this.cast::<Stream>() };
    let previous = stream.refs.fetch_sub(1, Ordering::AcqRel);
    if previous == 1 {
        unsafe { drop(Box::from_raw(this.cast::<Stream>())) };
    }
    previous - 1
}
unsafe extern "C" fn read(this: *mut c_void, buffer: *mut c_void, size: i32) -> i32 {
    let Ok(mut state) = unsafe { &*this.cast::<Stream>() }.state.lock() else {
        return -1;
    };
    if state.failed || matches!(state.data, Data::Closed) {
        return -1;
    }
    if size < 0 || size > 0 && buffer.is_null() {
        return state.fail(-1);
    }
    let count = (size as usize).min(state.bytes().len().saturating_sub(state.position));
    if count > 0 {
        unsafe {
            ptr::copy_nonoverlapping(
                state.bytes().as_ptr().add(state.position),
                buffer.cast(),
                count,
            );
        }
    }
    state.position += count;
    count as i32
}
unsafe extern "C" fn write(this: *mut c_void, buffer: *const c_void, size: i32) -> i32 {
    let Ok(mut state) = unsafe { &*this.cast::<Stream>() }.state.lock() else {
        return -1;
    };
    if state.failed || matches!(state.data, Data::Closed) {
        return -1;
    }
    if size < 0 || size > 0 && buffer.is_null() {
        return state.fail(-1);
    }
    let position = state.position;
    let Some(end) = position
        .checked_add(size as usize)
        .filter(|end| *end <= state.limit)
    else {
        return state.fail(-1);
    };
    let Data::Capture(bytes) = &mut state.data else {
        return state.fail(-1);
    };
    if end > bytes.len() {
        bytes.resize(end, 0);
    }
    if size > 0 {
        unsafe {
            ptr::copy_nonoverlapping(
                buffer.cast::<u8>(),
                bytes.as_mut_ptr().add(position),
                size as usize,
            );
        }
    }
    state.position = end;
    size
}
unsafe extern "C" fn tell(this: *mut c_void) -> i64 {
    let Ok(state) = unsafe { &*this.cast::<Stream>() }.state.lock() else {
        return -1;
    };
    if state.failed || matches!(state.data, Data::Closed) {
        -1
    } else {
        state.position as i64
    }
}
unsafe extern "C" fn seekable(this: *mut c_void) -> u8 {
    let Ok(state) = unsafe { &*this.cast::<Stream>() }.state.lock() else {
        return 0;
    };
    u8::from(!state.failed && !matches!(state.data, Data::Closed))
}
unsafe extern "C" fn seek(this: *mut c_void, position: i64, mode: i32) -> i64 {
    let Ok(mut state) = unsafe { &*this.cast::<Stream>() }.state.lock() else {
        return -1;
    };
    if state.failed || matches!(state.data, Data::Closed) {
        return -1;
    }
    let base = match mode {
        0 => 0,
        1 => state.position as i64,
        2 => state.bytes().len() as i64,
        _ => return state.fail(-1),
    };
    let Some(end) = base
        .checked_add(position)
        .and_then(|p| usize::try_from(p).ok())
        .filter(|p| *p <= state.limit)
    else {
        return state.fail(-1);
    };
    state.position = end;
    end as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_is_seekable_and_freezes_an_independent_copy() {
        let stream = MemoryStream::capture(8);
        assert_eq!(
            unsafe { write(stream.raw(), b"abcd".as_ptr().cast(), 4) },
            4
        );
        assert_eq!(unsafe { seek(stream.raw(), -2, 1) }, 2);
        assert_eq!(unsafe { write(stream.raw(), b"X".as_ptr().cast(), 1) }, 1);
        assert_eq!(stream.finish_capture().unwrap(), b"abXd");
        assert_eq!(unsafe { write(stream.raw(), b"Y".as_ptr().cast(), 1) }, -1);
        assert_eq!(unsafe { tell(stream.raw()) }, -1);
    }
    #[test]
    fn oversize_and_invalid_seek_are_sticky_failures() {
        for invalid_seek in [false, true] {
            let stream = MemoryStream::capture(4);
            assert_eq!(unsafe { write(stream.raw(), b"ok".as_ptr().cast(), 2) }, 2);
            if invalid_seek {
                assert_eq!(unsafe { seek(stream.raw(), -1, 0) }, -1);
            } else {
                assert_eq!(
                    unsafe { write(stream.raw(), b"oversize".as_ptr().cast(), 8) },
                    -1
                );
            }
            assert_eq!(unsafe { write(stream.raw(), b"x".as_ptr().cast(), 1) }, -1);
            assert!(stream.finish_capture().is_err());
        }
    }
    #[test]
    fn restore_is_read_only_and_reports_eof_without_corrupting_input() {
        let bytes: Arc<[u8]> = Arc::from(&b"state"[..]);
        let stream = MemoryStream::restore(bytes.clone());
        let mut output = [0; 8];
        assert_eq!(
            unsafe { read(stream.raw(), output.as_mut_ptr().cast(), 8) },
            5
        );
        assert_eq!(&output[..5], b"state");
        assert_eq!(
            unsafe { read(stream.raw(), output.as_mut_ptr().cast(), 1) },
            0
        );
        assert_eq!(unsafe { seek(stream.raw(), 0, 0) }, 0);
        assert!(stream.finish_restore().is_ok());
        assert_eq!(&*bytes, b"state");
        let stream = MemoryStream::restore(bytes);
        assert_eq!(unsafe { write(stream.raw(), b"x".as_ptr().cast(), 1) }, -1);
        assert!(stream.finish_restore().is_err());
    }
    #[test]
    fn retained_native_stream_is_sealed_after_request_owner_drops() {
        let stream = MemoryStream::capture(4);
        let raw = stream.raw();
        unsafe { retain(raw) };
        drop(stream);
        assert_eq!(unsafe { write(raw, b"x".as_ptr().cast(), 1) }, -1);
        assert_eq!(unsafe { seekable(raw) }, 0);
        assert_eq!(unsafe { release(raw) }, 0);
    }
    #[test]
    fn foreign_stream_callbacks_can_run_on_another_thread() {
        let stream = MemoryStream::capture(16);
        let raw = stream.raw() as usize;
        unsafe { retain(raw as *mut c_void) };
        std::thread::spawn(move || {
            let raw = raw as *mut c_void;
            assert_eq!(unsafe { write(raw, b"worker".as_ptr().cast(), 6) }, 6);
            unsafe { release(raw) };
        })
        .join()
        .unwrap();
        assert_eq!(stream.finish_capture().unwrap(), b"worker");
    }
}
