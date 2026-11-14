use super::*;

#[test]
fn details_preserve_native_precision_and_each_fixed_state_flag() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    for (index, mask) in [0x80, 0x100, 0x800, 0x1000, 0x10000]
        .into_iter()
        .enumerate()
    {
        f.p().details.state.set(mask);
        f.p().details.precision.set(10_000);
        let details = p.details(&mut f.guard()).unwrap();
        assert_eq!(details.precision, 10_000);
        for (flag_index, value) in [
            details.storable,
            details.store_list_value,
            details.mutable,
            details.read_only,
            details.can_undo,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(value, flag_index == index);
        }
        assert!(!details.list_interface);
    }
    f.p().details.precision.set(-1);
    assert_eq!(p.details(&mut f.guard()).unwrap().precision, -1);
    assert_eq!(f.p().writes.get(), 0);
    assert!(f.p().choice_calls.borrow().is_empty());
    assert_eq!(f.p().refs.get(), 2);
}

#[test]
fn details_query_capability_does_not_infer_it_from_kind_or_float_bounds() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    f.p().details.list_interface.set(true);
    let details = p.details(&mut f.guard()).unwrap();
    assert!(details.list_interface);
    assert_eq!(f.p().refs.get(), 2);
    f.p().details.list_query_error.set(Some(-123));
    assert_eq!(
        p.details(&mut f.guard()),
        Err(Error::NativeFailure("IListParameter", -123))
    );
    assert_eq!(f.p().writes.get(), 0);
}

#[test]
fn details_reject_context_loss_in_getter_query_and_temporary_release() {
    let env = TextEnv::new();
    for scenario in 0..3 {
        let (f, p) = float_fixture(&env);
        match scenario {
            0 => f.p().details.stop_on_precision.set(true),
            1 => f.p().details.stop_on_query.set(true),
            _ => {
                f.p().details.list_interface.set(true);
                f.p().details.stop_on_release_at.set(Some(3));
            }
        }
        assert!(matches!(p.details(&mut f.guard()), Err(Error::Guard(_))));
        assert_eq!(f.p().refs.get(), 2);
        assert_eq!(f.p().writes.get(), 0);
    }
}

#[test]
fn stale_initial_context_prevents_all_details_and_original_queries() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    f.p().context.set(false);
    assert!(matches!(p.details(&mut f.guard()), Err(Error::Guard(_))));
    assert!(matches!(
        p.original_chain(&mut f.guard()),
        Err(Error::Guard(_))
    ));
    assert_eq!(f.p().details.precision_calls.get(), 0);
    assert_eq!(f.p().details.original_calls.get(), 0);
    assert_eq!(f.p().reads.get(), 0);
    assert_eq!(f.p().refs.get(), 2);
}

#[test]
fn self_original_is_a_complete_single_observation_and_releases_all_pins() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    let chain = p.original_chain(&mut f.guard()).unwrap();
    assert_eq!(chain.end, OriginalChainEnd::SelfReference);
    assert_eq!(chain.parameters.len(), 1);
    assert_eq!(
        chain.parameters[0].description.raw.as_ref().unwrap().value,
        Value::Float(1.0)
    );
    assert_eq!(f.p().details.original_calls.get(), 1);
    assert_eq!(f.p().refs.get(), 2);
    assert_eq!(f.p().writes.get(), 0);
}

#[test]
fn alias_chain_preserves_each_representation_without_guessing_model_choices() {
    let env = TextEnv::new();
    let (outer_f, outer) = Fixture::new(
        &env,
        4,
        FakeValue::Number(Number::Float(5000.0)),
        Number::Float(0.0),
        Number::Float(10000.0),
        None,
    );
    let (inner_f, _inner) = Fixture::new(
        &env,
        4,
        FakeValue::Number(Number::Integer(1)),
        Number::Integer(0),
        Number::Integer(3),
        None,
    );
    outer_f.p().details.original.set(Some(inner_f.pointer));
    outer_f.p().details.precision.set(10_000);
    inner_f.p().details.list_interface.set(true);
    inner_f.p().details.precision.set(3);
    let chain = outer.original_chain(&mut outer_f.guard()).unwrap();
    assert_eq!(chain.end, OriginalChainEnd::SelfReference);
    assert_eq!(chain.parameters.len(), 2);
    let facade = &chain.parameters[0];
    let original = &chain.parameters[1];
    assert_eq!(facade.description.kind, Kind::List);
    assert_eq!(
        facade.description.max.as_ref().unwrap().value,
        Value::Float(10000.0)
    );
    assert_eq!(
        original.description.max.as_ref().unwrap().value,
        Value::Integer(3)
    );
    assert_eq!(
        original.description.raw.as_ref().unwrap().value,
        Value::Integer(1)
    );
    assert_eq!(facade.details.precision, 10_000);
    assert!(!facade.details.list_interface);
    assert!(original.details.list_interface);
    for f in [&outer_f, &inner_f] {
        assert_eq!(f.p().refs.get(), 2);
        assert_eq!(f.p().writes.get(), 0);
        assert!(f.p().choice_calls.borrow().is_empty());
    }
}

#[test]
fn canonical_identity_deduplicates_different_parameter_views_as_self() {
    let env = TextEnv::new();
    let (a_f, a) = float_fixture(&env);
    let (view_f, _view) = float_fixture(&env);
    a_f.p().details.original.set(Some(view_f.pointer));
    view_f.p().details.canonical.set(a_f.pointer);
    let chain = a.original_chain(&mut a_f.guard()).unwrap();
    assert_eq!(chain.parameters.len(), 1);
    assert_eq!(chain.end, OriginalChainEnd::SelfReference);
    assert_eq!(view_f.p().reads.get(), 0);
    assert_eq!(view_f.p().refs.get(), 2);
    assert_eq!(a_f.p().refs.get(), 2);
}

#[test]
fn canonical_cycle_is_reported_without_repeating_or_following_that_entry() {
    let env = TextEnv::new();
    let (a_f, a) = float_fixture(&env);
    let (b_f, _b) = float_fixture(&env);
    let (a_view_f, _a_view) = float_fixture(&env);
    a_f.p().details.original.set(Some(b_f.pointer));
    b_f.p().details.original.set(Some(a_view_f.pointer));
    a_view_f.p().details.canonical.set(a_f.pointer);
    let chain = a.original_chain(&mut a_f.guard()).unwrap();
    assert_eq!(chain.parameters.len(), 2);
    assert_eq!(chain.end, OriginalChainEnd::Cycle { index: 0 });
    assert_eq!(a_view_f.p().details.original_calls.get(), 0);
    for f in [&a_f, &b_f, &a_view_f] {
        assert_eq!(f.p().refs.get(), 2);
    }
}

#[test]
fn missing_original_is_explicit_and_canonical_identity_failures_are_errors() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    f.p().details.original.set(Some(std::ptr::null_mut()));
    let chain = p.original_chain(&mut f.guard()).unwrap();
    assert_eq!(chain.end, OriginalChainEnd::MissingOriginal);
    assert_eq!(chain.parameters.len(), 1);
    for failure in [ffi::NO_INTERFACE, -123] {
        f.p().details.unknown_query_error.set(Some(failure));
        let error = p.original_chain(&mut f.guard()).unwrap_err();
        assert_eq!(
            error,
            if failure == ffi::NO_INTERFACE {
                Error::MissingInterface("IUnknown")
            } else {
                Error::NativeFailure("IUnknown", -123)
            }
        );
        assert_eq!(f.p().refs.get(), 2);
    }
    f.p().details.unknown_query_error.set(None);
    f.p().details.null_unknown.set(true);
    assert_eq!(
        p.original_chain(&mut f.guard()),
        Err(Error::MissingInterface("IUnknown"))
    );
    assert_eq!(f.p().refs.get(), 2);
}

#[test]
fn eight_observation_limit_never_follows_the_eighth_original() {
    let env = TextEnv::new();
    let pairs: Vec<_> = (0..9).map(|_| float_fixture(&env)).collect();
    for pair in pairs.windows(2) {
        pair[0].0.p().details.original.set(Some(pair[1].0.pointer));
    }
    let chain = pairs[0].1.original_chain(&mut pairs[0].0.guard()).unwrap();
    assert_eq!(chain.parameters.len(), 8);
    assert_eq!(chain.end, OriginalChainEnd::Limit);
    assert_eq!(pairs[7].0.p().details.original_calls.get(), 0);
    assert_eq!(pairs[8].0.p().reads.get(), 0);
    for (f, _) in &pairs {
        assert_eq!(f.p().refs.get(), 2);
    }
}

#[test]
fn borrowed_original_is_owned_before_the_first_reentrant_guard() {
    let env = TextEnv::new();
    let (outer_f, outer) = float_fixture(&env);
    let (inner_f, inner) = float_fixture(&env);
    outer_f.p().details.original.set(Some(inner_f.pointer));
    let mut inner_owner = Some(inner);
    let mut observed = false;
    let mut guard = || {
        if outer_f.p().details.original_calls.get() != 0 {
            if !observed {
                assert!(
                    inner_f.p().refs.get() >= 3,
                    "the borrowed original must already be retained"
                );
            }
            observed = true;
            drop(inner_owner.take());
            Err("document replaced during guard".to_owned())
        } else {
            Ok(())
        }
    };
    assert!(matches!(
        outer.original_chain(&mut guard),
        Err(Error::Guard(_))
    ));
    assert!(observed);
    assert_eq!(inner_f.p().refs.get(), 1);
    assert_eq!(outer_f.p().refs.get(), 2);
}

#[test]
fn original_getter_and_final_native_release_context_loss_discard_the_chain() {
    let env = TextEnv::new();
    for during_release in [false, true] {
        let (f, p) = float_fixture(&env);
        if during_release {
            f.p().details.stop_on_release_at.set(Some(5));
        } else {
            f.p().details.stop_on_original.set(true);
        }
        assert!(matches!(
            p.original_chain(&mut f.guard()),
            Err(Error::Guard(_))
        ));
        assert_eq!(f.p().refs.get(), 2);
        assert_eq!(f.p().writes.get(), 0);
    }
}

#[test]
fn original_snapshot_cleanup_and_original_identity_failure_discard_all_pins() {
    let env = TextEnv::new();
    let (outer_f, outer) = float_fixture(&env);
    let (inner_f, _inner) = float_fixture(&env);
    outer_f.p().details.original.set(Some(inner_f.pointer));
    inner_f.p().details.unknown_query_error.set(Some(-123));
    assert_eq!(
        outer.original_chain(&mut outer_f.guard()),
        Err(Error::NativeFailure("IUnknown", -123))
    );
    assert_eq!(outer_f.p().refs.get(), 2);
    assert_eq!(inner_f.p().refs.get(), 2);
    inner_f.p().details.unknown_query_error.set(None);
    inner_f.p().details.description_stop_on_drop.set(true);
    let mut guard = || {
        if outer_f.p().context.get() && inner_f.p().context.get() {
            Ok(())
        } else {
            Err("snapshot release replaced context".to_owned())
        }
    };
    assert!(matches!(
        outer.original_chain(&mut guard),
        Err(Error::Guard(_))
    ));
    assert_eq!(outer_f.p().refs.get(), 2);
    assert_eq!(inner_f.p().refs.get(), 2);
    assert_eq!(inner_f.p().details.original_calls.get(), 0);
    assert_eq!(outer_f.p().writes.get() + inner_f.p().writes.get(), 0);
}

#[test]
fn original_snapshot_preserves_missing_properties_instead_of_synthesizing_values() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    f.p().false_property.set(true);
    let chain = p.original_chain(&mut f.guard()).unwrap();
    let description = &chain.parameters[0].description;
    assert_eq!(description.raw, None);
    assert_eq!(description.min, None);
    assert_eq!(description.max, None);
    assert_eq!(description.default_value, None);
    assert_eq!(description.display, "1");
    assert_eq!(description.normalized, Normalized::Finite(0.5));
    assert!(!description.support.raw_number);
    assert_eq!(chain.end, OriginalChainEnd::SelfReference);
    assert_eq!(f.p().refs.get(), 2);
}
