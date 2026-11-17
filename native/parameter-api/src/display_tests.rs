use super::*;

thread_local! {
    static MODE: Cell<u32> = const { Cell::new(0) };
    static CALLS: Cell<u32> = const { Cell::new(0) };
}
unsafe extern "C" fn display(pointer: *mut c_void, output: *mut StringRepr) {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    unsafe { p_display(pointer, output) };
    let parameter = unsafe { fp(pointer) };
    let string = unsafe { fs((*output).object) };
    match MODE.with(Cell::get) {
        1 => *string.units.borrow_mut() = vec![0xd800],
        2 => string.fail_copy.set(true),
        3 => string.skip_termination.set(true),
        4 => parameter.context.set(false),
        5 => *string.stop_on_copy.borrow_mut() = Some(parameter.context.clone()),
        6 | 7 => {
            *string.stop_on_release.borrow_mut() = Some(parameter.context.clone());
            string.final_release_only.set(MODE.with(Cell::get) == 7);
        }
        8 => unsafe { (*pointer.cast::<FakeParameter>()).name.text = c"replaced".as_ptr() },
        _ => {}
    }
}
fn make_fixture(env: &TextEnv, value: &str) -> (Fixture, RawParameter, Box<[usize; 46]>) {
    let (fixture, parameter) = Fixture::new(
        env,
        3,
        FakeValue::Text(value.into()),
        Number::Integer(0),
        Number::Integer(0),
        None,
    );
    let mut table = Box::new(unsafe { *fixture.p().table.cast::<[usize; 46]>() });
    table[44] = display as *const () as usize;
    unsafe {
        (*fixture.pointer.cast::<FakeParameter>()).name.text = c"presetName".as_ptr();
        (*fixture.pointer.cast::<FakeParameter>()).table = table.as_ptr();
    }
    MODE.with(|mode| mode.set(0));
    CALLS.with(|calls| calls.set(0));
    (fixture, parameter, table)
}

#[test]
fn display_only_reads_exact_unicode_empty_and_literal_asterisk_without_properties_or_edits() {
    let env = TextEnv::new();
    for value in [
        "".into(),
        "Initial* 한글 🎸".into(),
        "가".repeat(MAX_TEXT_UNITS),
    ] {
        let (fixture, parameter, table) = make_fixture(&env, &value);
        // A property QI or access/formatter call would invalidate this context.
        fixture.p().details.stop_on_query.set(true);
        fixture.p().stop_on_enabled.set(true);
        assert_eq!(
            parameter
                .string_display("presetName", &mut fixture.guard())
                .unwrap(),
            value
        );
        assert!(fixture.p().context.get());
        assert_eq!(fixture.p().reads.get(), 0);
        assert_eq!(fixture.p().writes.get(), 0);
        assert!(fixture.p().choice_calls.borrow().is_empty());
        assert_eq!(CALLS.with(Cell::get), 1);
        drop(parameter);
        drop(fixture);
        drop(table);
    }
}

#[test]
fn display_rejects_wrong_name_type_oversize_and_malformed_native_text() {
    let env = TextEnv::new();
    let (fixture, parameter, table) = make_fixture(&env, "ok");
    assert_eq!(
        parameter.string_display("presetMenu", &mut fixture.guard()),
        Err(Error::MetadataChanged)
    );
    assert_eq!(CALLS.with(Cell::get), 0);
    unsafe { (*fixture.pointer.cast::<FakeParameter>()).kind = 4 };
    assert_eq!(
        parameter.string_display("presetName", &mut fixture.guard()),
        Err(Error::UnsupportedKind)
    );
    assert_eq!(CALLS.with(Cell::get), 0);
    drop(parameter);
    drop(fixture);
    drop(table);
    for mode in 0..=3 {
        let value = if mode == 0 {
            "a".repeat(MAX_TEXT_UNITS + 1)
        } else {
            "ok".into()
        };
        let (fixture, parameter, table) = make_fixture(&env, &value);
        MODE.with(|current| current.set(mode));
        assert!(
            parameter
                .string_display("presetName", &mut fixture.guard())
                .is_err()
        );
        assert_eq!(fixture.p().writes.get(), 0);
        assert_eq!(fixture.p().refs.get(), 2);
        drop(parameter);
        drop(fixture);
        drop(table);
    }
}

#[test]
fn display_withholds_text_after_get_copy_temporary_or_final_string_release_context_loss() {
    let env = TextEnv::new();
    for mode in 4..=8 {
        let (fixture, parameter, table) = make_fixture(&env, "observed before reentry");
        MODE.with(|current| current.set(mode));
        assert!(
            parameter
                .string_display("presetName", &mut fixture.guard())
                .is_err(),
            "mode{mode}"
        );
        assert_eq!(fixture.p().writes.get(), 0);
        assert_eq!(fixture.p().reads.get(), 0);
        assert_eq!(fixture.p().refs.get(), 2);
        drop(parameter);
        drop(fixture);
        drop(table);
    }
}

#[test]
fn display_fails_closed_at_every_guard_including_out_string_cleanup() {
    let env = TextEnv::new();
    let (fixture, parameter, table) = make_fixture(&env, "Initial");
    let mut calls = 0;
    parameter
        .string_display("presetName", &mut || {
            calls += 1;
            Ok(())
        })
        .unwrap();
    drop(parameter);
    drop(fixture);
    drop(table);
    for stop in 1..=calls {
        let (fixture, parameter, table) = make_fixture(&env, "Initial");
        let mut observed = 0;
        assert!(
            parameter
                .string_display("presetName", &mut || {
                    observed += 1;
                    if observed >= stop {
                        Err("context changed".into())
                    } else {
                        Ok(())
                    }
                })
                .is_err(),
            "guard{stop}"
        );
        assert_eq!(fixture.p().writes.get(), 0);
        drop(parameter);
        drop(fixture);
        drop(table);
    }
}

#[test]
fn final_out_string_cleanup_cannot_hide_a_same_route_parameter_signature_change() {
    let env = TextEnv::new();
    let (fixture, parameter, table) = make_fixture(&env, "Initial");
    let result = parameter.string_display("presetName", &mut || {
        if env.stats.destroyed.get() > 0 {
            unsafe { (*fixture.pointer.cast::<FakeParameter>()).name.text = c"replaced".as_ptr() };
        }
        Ok(())
    });
    assert_eq!(result, Err(Error::MetadataChanged));
    assert_eq!(fixture.p().writes.get(), 0);
    drop(parameter);
    drop(fixture);
    drop(table);
}
