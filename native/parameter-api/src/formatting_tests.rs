use super::*;

#[test]
fn incomparable_defaults_are_rejected_before_any_format_call() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    f.p().format.enabled.set(true);
    let object = string_new("default owner", env.stats.clone());
    for default in [
        DefaultCarrier::Object(std::ptr::null_mut()),
        DefaultCarrier::Object(object),
        DefaultCarrier::Unsupported { tag: 5, payload: 1 },
        DefaultCarrier::Unsupported {
            tag: 8,
            payload: u64::MAX,
        },
    ] {
        f.p().format.default_carrier.set(Some(default));
        assert_eq!(
            p.format_values(&[Number::Float(0.5)], &mut f.guard()),
            Err(Error::InvalidData(
                "default value cannot be compared completely"
            ))
        );
        assert!(f.p().format.values.borrow().is_empty());
        assert_eq!(f.p().writes.get(), 0);
        assert_eq!(f.p().refs.get(), 2);
        assert_eq!(unsafe { fs(object) }.refs.get(), 1);
    }
    unsafe { string_release(object) };
}

#[test]
fn known_nonfinite_default_bits_are_comparable_and_changes_are_rejected() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    f.p().format.enabled.set(true);
    let bits = 0x7ff8000000000123;
    f.p()
        .format
        .default_carrier
        .set(Some(DefaultCarrier::FloatBits(bits)));
    let formatted = p
        .format_values(&[Number::Float(0.5)], &mut f.guard())
        .unwrap();
    assert_eq!(
        formatted.before.default_value.as_ref().unwrap().value,
        Value::NonFiniteFloat { bits }
    );
    assert_eq!(
        formatted.after.default_value,
        formatted.before.default_value
    );
    assert!(formatted.current_unchanged);
    f.p().format.values.borrow_mut().clear();
    f.p()
        .format
        .default_after_get
        .set(Some(DefaultCarrier::FloatBits(bits + 1)));
    assert_eq!(
        p.format_values(&[Number::Float(0.5), Number::Float(1.0)], &mut f.guard()),
        Err(Error::MetadataChanged)
    );
    assert_eq!(f.p().format.values.borrow().len(), 1);
    assert_eq!(f.p().writes.get(), 0);
    assert_eq!(f.p().refs.get(), 2);
}

#[test]
fn explicit_float_values_format_the_input_in_order_without_changing_selection() {
    let env = TextEnv::new();
    let (f, p) = Fixture::new(
        &env,
        4,
        FakeValue::Number(Number::Float(2.0)),
        Number::Float(0.0),
        Number::Float(5.0),
        None,
    );
    f.p().format.enabled.set(true);
    f.p().details.precision.set(5);
    let requested = [Number::Float(0.5), Number::Integer(4), Number::Float(0.5)];
    let result = p.format_values(&requested, &mut f.guard()).unwrap();
    assert!(result.current_unchanged);
    assert_eq!(
        result
            .values
            .iter()
            .map(|item| item.requested)
            .collect::<Vec<_>>(),
        requested
    );
    assert_eq!(
        result
            .values
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        [
            "float:3FE0000000000000",
            "float:4010000000000000",
            "float:3FE0000000000000",
        ]
    );
    assert_eq!(
        *f.p().format.values.borrow(),
        [Number::Float(0.5), Number::Float(4.0), Number::Float(0.5)]
    );
    assert_eq!(
        result.values[1].native_value,
        NativeValue {
            variant_tag: 2,
            value: Value::Float(4.0)
        }
    );
    assert_eq!(result.before.raw.as_ref().unwrap().value, Value::Float(2.0));
    assert_eq!(result.after.raw, result.before.raw);
    assert_eq!(result.after.normalized, result.before.normalized);
    assert!(f.p().choice_calls.borrow().is_empty());
    assert_eq!(f.p().writes.get(), 0);
    assert_eq!(f.p().refs.get(), 2);
}

#[test]
fn format_integer_carriers_remain_exact_and_readonly_disabled_is_supported() {
    let env = TextEnv::new();
    let (f, p) = Fixture::new(
        &env,
        1,
        FakeValue::Number(Number::Integer(0)),
        Number::Integer(i64::MIN),
        Number::Integer(i64::MAX),
        None,
    );
    f.p().format.enabled.set(true);
    f.p().enabled.set(false);
    f.p().readonly.set(true);
    let result = p
        .format_values(
            &[Number::Integer(i64::MAX), Number::Float(2.0)],
            &mut f.guard(),
        )
        .unwrap();
    assert_eq!(result.values[0].label, "integer:9223372036854775807");
    assert_eq!(
        result.values[0].native_value.value,
        Value::Integer(i64::MAX)
    );
    assert_eq!(result.values[1].label, "integer:2");
    assert_eq!(result.values[1].native_value.value, Value::Integer(2));
    assert!(!result.before.enabled);
    assert!(result.before.read_only);
    assert!(result.current_unchanged);
    assert_eq!(f.p().writes.get(), 0);
}

#[test]
fn format_rejects_whole_invalid_batches_before_get_string() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    f.p().format.enabled.set(true);
    for (values, expected) in [
        (
            vec![],
            Error::InvalidData("format values must contain 1..32 numbers"),
        ),
        (
            vec![Number::Integer(1); 33],
            Error::InvalidData("format values must contain 1..32 numbers"),
        ),
        (vec![Number::Float(f64::NAN)], Error::NonFinite),
        (vec![Number::Float(f64::INFINITY)], Error::NonFinite),
        (
            vec![Number::Float(0.5), Number::Float(3.0)],
            Error::OutOfRange,
        ),
        (vec![Number::Integer(9007199254740993)], Error::TypeMismatch),
    ] {
        assert_eq!(p.format_values(&values, &mut f.guard()), Err(expected));
        assert!(f.p().format.values.borrow().is_empty());
    }
    let (integer_f, integer) = Fixture::new(
        &env,
        1,
        FakeValue::Number(Number::Integer(1)),
        Number::Integer(0),
        Number::Integer(2),
        None,
    );
    integer_f.p().format.enabled.set(true);
    assert_eq!(
        integer.format_values(&[Number::Float(0.5)], &mut integer_f.guard()),
        Err(Error::TypeMismatch)
    );
    assert!(integer_f.p().format.values.borrow().is_empty());
    for kind in [3, 5, 6, 7, 8, 10, 99] {
        let (f, p) = Fixture::new(
            &env,
            kind,
            FakeValue::Number(Number::Integer(1)),
            Number::Integer(0),
            Number::Integer(2),
            None,
        );
        f.p().format.enabled.set(true);
        assert_eq!(
            p.format_values(&[Number::Integer(1)], &mut f.guard()),
            Err(Error::UnsupportedKind)
        );
        assert!(f.p().format.values.borrow().is_empty());
    }
    let (object_f, object) = Fixture::new(
        &env,
        2,
        FakeValue::Object(std::ptr::null_mut()),
        Number::Integer(0),
        Number::Integer(2),
        None,
    );
    object_f.p().format.enabled.set(true);
    assert_eq!(
        object.format_values(&[Number::Integer(1)], &mut object_f.guard()),
        Err(Error::MissingValue("finite numeric value"))
    );
    assert!(object_f.p().format.values.borrow().is_empty());
}

#[test]
fn format_maximum_batch_and_numeric_kinds_do_not_require_a_list_interface() {
    let env = TextEnv::new();
    for kind in [0, 1, 2, 4, 9] {
        let (f, p) = Fixture::new(
            &env,
            kind,
            FakeValue::Number(Number::Integer(1)),
            Number::Integer(0),
            Number::Integer(2),
            None,
        );
        f.p().format.enabled.set(true);
        let result = p
            .format_values(&[Number::Integer(1); 32], &mut f.guard())
            .unwrap();
        assert_eq!(result.values.len(), 32);
        assert!(result.values.iter().all(|value| value.label == "integer:1"));
        assert_eq!(f.p().writes.get(), 0);
    }
}

#[test]
fn format_rejects_metadata_current_value_and_normalized_changes() {
    let env = TextEnv::new();
    for scenario in 0..7 {
        let (f, p) = float_fixture(&env);
        f.p().format.enabled.set(true);
        match scenario {
            0 => f.p().choice_max_after_get.set(Some(Number::Float(3.0))),
            1 => {
                *f.p().format.value_after_get.borrow_mut() =
                    Some(FakeValue::Number(Number::Float(0.5)))
            }
            2 => f.p().format.normalized_after_get.set(Some(0.75)),
            3 => f.p().format.state_after_get.set(Some(0x800)),
            4 => {
                *f.p().value.borrow_mut() = FakeValue::Number(Number::Float(-0.0));
                *f.p().format.value_after_get.borrow_mut() =
                    Some(FakeValue::Number(Number::Float(0.0)));
            }
            5 => {
                f.p().format.normalized_override.set(Some(-0.0));
                f.p().format.normalized_after_get.set(Some(0.0));
            }
            _ => f.p().format.precision_after_get.set(Some(5)),
        }
        assert_eq!(
            p.format_values(&[Number::Float(0.25), Number::Float(1.0)], &mut f.guard()),
            Err(Error::MetadataChanged)
        );
        assert_eq!(f.p().format.values.borrow().len(), 1);
        assert_eq!(f.p().writes.get(), 0);
        assert_eq!(f.p().refs.get(), 2);
    }
}

#[test]
fn format_copy_errors_and_context_reentry_never_publish_partial_values() {
    let env = TextEnv::new();
    for scenario in 0..7 {
        let (f, p) = float_fixture(&env);
        f.p().format.enabled.set(true);
        match scenario {
            0 => f.p().choice_copy_fails.set(true),
            1 => f.p().choice_invalid_utf16.set(true),
            2 => f.p().choice_skip_termination.set(true),
            3 => f.p().choice_stop_on_get.set(true),
            4 => f.p().choice_stop_on_copy.set(true),
            5 => f.p().choice_stop_on_drop.set(true),
            _ => f.p().context.set(false),
        }
        let error = p
            .format_values(&[Number::Float(0.25), Number::Float(1.0)], &mut f.guard())
            .unwrap_err();
        if scenario >= 3 {
            assert!(matches!(error, Error::Guard(_)));
        }
        assert_eq!(
            f.p().format.values.borrow().len(),
            if scenario == 6 { 0 } else { 1 }
        );
        assert_eq!(f.p().writes.get(), 0);
        assert_eq!(f.p().refs.get(), 2);
    }
}

#[test]
fn get_string_void_refusal_remains_an_observed_empty_string() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    f.p().format.enabled.set(true);
    f.p().format.refuse_output.set(true);
    let result = p
        .format_values(&[Number::Float(0.25)], &mut f.guard())
        .unwrap();
    assert_eq!(result.values[0].label, "");
    assert!(result.current_unchanged);
    assert_eq!(f.p().format.values.borrow().len(), 1);
    assert_eq!(f.p().writes.get(), 0);
}
