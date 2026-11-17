use super::*;
use std::cell::RefCell;

#[derive(Default)]
struct Reply {
    size: u32,
    returned_size: Option<u32>,
    size_status: i32,
    data_status: i32,
    data: Vec<u8>,
    data_calls: usize,
    address: Option<(u32, u32, u32, u32)>,
}
thread_local! { static REPLY: RefCell<Reply> = RefCell::new(Reply::default()); }

unsafe extern "C" fn size(
    object: u32,
    address: *const Address,
    _: u32,
    _: *const c_void,
    value: *mut u32,
) -> i32 {
    REPLY.with(|cell| {
        let mut reply = cell.borrow_mut();
        let address = unsafe { *address };
        reply.address = Some((object, address.selector, address.scope, address.element));
        unsafe { *value = reply.size };
        reply.size_status
    })
}
unsafe extern "C" fn data(
    _: u32,
    _: *const Address,
    _: u32,
    _: *const c_void,
    size: *mut u32,
    value: *mut c_void,
) -> i32 {
    REPLY.with(|cell| {
        let mut reply = cell.borrow_mut();
        reply.data_calls += 1;
        assert_eq!(value as usize % mem::align_of::<AudioBuffer>(), 0);
        let capacity = unsafe { *size } as usize;
        assert!(reply.data.len() <= capacity);
        unsafe {
            ptr::copy_nonoverlapping(reply.data.as_ptr(), value.cast(), reply.data.len());
            *size = reply.returned_size.unwrap_or(reply.data.len() as u32);
        }
        reply.data_status
    })
}
fn api(reply: Reply) -> AudioApi {
    REPLY.with(|cell| *cell.borrow_mut() = reply);
    AudioApi { size, data }
}
fn value(data: Vec<u8>) -> Reply {
    Reply {
        size: data.len() as u32,
        data,
        ..Reply::default()
    }
}

#[test]
fn native_audio_reader_preserves_status_and_refuses_size_races_before_parsing() {
    let native = api(Reply {
        size_status: -123,
        ..Reply::default()
    });
    assert_eq!(
        native.integer(7, 19),
        Observation::Unavailable {
            os_status: Some(-123),
            reason: "property_read_failed"
        }
    );
    REPLY.with(|cell| assert_eq!(cell.borrow().data_calls, 0));
    let native = api(Reply {
        size: 8,
        ..Reply::default()
    });
    assert!(native.integer(7, 19).value().is_none());
    REPLY.with(|cell| assert_eq!(cell.borrow().data_calls, 0));
    let native = api(Reply {
        size: 4,
        returned_size: Some(8),
        ..Reply::default()
    });
    assert!(native.integer(7, 19).value().is_none());
    let native = api(Reply {
        size: 4,
        data_status: -456,
        ..Reply::default()
    });
    assert_eq!(
        native.integer(7, 19),
        Observation::Unavailable {
            os_status: Some(-456),
            reason: "property_read_failed"
        }
    );
}

#[test]
fn native_audio_reader_passes_exact_address_and_reads_native_numeric_bits() {
    let native = api(value(48_000f64.to_ne_bytes().to_vec()));
    assert_eq!(native.rate(7).value(), Some(&48_000.0));
    REPLY.with(|cell| {
        assert_eq!(
            cell.borrow().address,
            Some((7, u32::from_be_bytes(*b"nsrt"), GLOBAL, 0))
        )
    });
    for bits in [
        f64::NAN.to_bits(),
        f64::INFINITY.to_bits(),
        (-1f64).to_bits(),
        0,
    ] {
        let native = api(value(bits.to_ne_bytes().to_vec()));
        assert!(native.rate(7).value().is_none());
    }
    let native = api(value(2u32.to_ne_bytes().to_vec()));
    assert!(native.boolean(7, 19).value().is_none());
}

#[test]
fn stream_configuration_bounds_do_not_dereference_audio_data_pointers() {
    assert_eq!(mem::size_of::<AudioBuffer>(), 16);
    assert_eq!(mem::offset_of!(AudioBufferList, buffers), 8);
    let mut bytes = vec![0u8; 8 + 2 * 16];
    bytes[..4].copy_from_slice(&2u32.to_ne_bytes());
    bytes[8..12].copy_from_slice(&2u32.to_ne_bytes());
    bytes[24..28].copy_from_slice(&1u32.to_ne_bytes());
    bytes[16..24].fill(255);
    bytes[32..40].fill(255);
    assert_eq!(parse_channels(&bytes).value(), Some(&3));
    assert_eq!(parse_channels(&0u32.to_ne_bytes()).value(), Some(&0));
    assert_eq!(parse_channels(&[0u8; 8]).value(), Some(&0));
    assert!(parse_channels(&bytes[..24]).value().is_none());
    bytes[..4].copy_from_slice(&u32::MAX.to_ne_bytes());
    assert!(parse_channels(&bytes).value().is_none());
    bytes[..4].copy_from_slice(&2u32.to_ne_bytes());
    bytes[8..12].copy_from_slice(&4096u32.to_ne_bytes());
    assert!(parse_channels(&bytes).value().is_none());
}

#[test]
fn native_audio_property_byte_limit_is_checked_before_data_read() {
    let native = api(Reply {
        size: MAX_PROPERTY_BYTES as u32 + 1,
        ..Reply::default()
    });
    assert!(
        native
            .raw(7, 19, GLOBAL, MAX_PROPERTY_BYTES)
            .value()
            .is_none()
    );
    REPLY.with(|cell| assert_eq!(cell.borrow().data_calls, 0));
    assert!(parse_ids(vec![0; 3], MAX_DEVICES).value().is_none());
    assert!(
        parse_ids(0u32.to_ne_bytes().to_vec(), MAX_DEVICES)
            .value()
            .is_none()
    );
}

#[test]
fn native_cf_string_conversion_bounds_and_preserves_unicode_without_truncating_nul() {
    for (text, accepted) in [
        ("Volt 한국어 🎹", true),
        ("", true),
        ("before\0after", false),
    ] {
        let object = unsafe {
            CFStringCreateWithBytes(ptr::null(), text.as_ptr(), text.len() as isize, UTF8, 0)
        };
        assert!(!object.is_null());
        let native = api(value((object as usize).to_ne_bytes().to_vec()));
        let observed = native.string(7, 19);
        if accepted {
            assert_eq!(observed.value().map(String::as_str), Some(text));
        } else {
            assert!(observed.value().is_none());
        }
    }
    let text = "a".repeat(MAX_STRING_UNITS + 1);
    let object = unsafe {
        CFStringCreateWithBytes(ptr::null(), text.as_ptr(), text.len() as isize, UTF8, 0)
    };
    let native = api(value((object as usize).to_ne_bytes().to_vec()));
    assert!(native.string(7, 19).value().is_none());
}
