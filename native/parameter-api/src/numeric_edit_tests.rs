use super::*;

fn callback(target: &RefCell<Option<EditCallback>>, action: impl Fn(&FakeParameter) + 'static) {
    *target.borrow_mut() = Some(Rc::new(action));
}
fn trace(f: &Fixture) -> Vec<&'static str> {
    f.p().edit.trace.borrow().clone()
}
fn assert_failure(attempt: &NumericEditAttempt, outcome: Outcome, phase: NumericEditPhase) {
    assert_eq!(attempt.result.as_ref().unwrap_err().outcome, outcome);
    assert_eq!(attempt.lease.phase(), phase);
}

#[test]
fn numeric_edit_updates_once_between_begin_and_end_and_keeps_binding_alive() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    let attempt = p.edit_raw_number(Number::Float(1.5), &mut f.guard());
    let receipt = attempt.result.as_ref().unwrap();
    assert!(receipt.verified);
    assert_eq!(trace(&f), ["begin", "value", "end"]);
    assert_eq!(*f.p().edit.updates.borrow(), [1]);
    assert_eq!(attempt.lease.phase(), NumericEditPhase::ClosedChecked);
    assert!(f.p().refs.get() > 2);
    attempt.lease.check(&mut f.guard()).unwrap();
    drop(attempt);
    assert_eq!(trace(&f), ["begin", "value", "end"]);
    assert_eq!(f.p().refs.get(), 1);
}

#[test]
fn numeric_edit_alias_calls_only_the_selected_receiver() {
    let env = TextEnv::new();
    let (outer, p) = float_fixture(&env);
    let (inner, _inner) = float_fixture(&env);
    outer.p().details.original.set(Some(inner.pointer));
    let attempt = p.edit_raw_number(Number::Float(1.5), &mut outer.guard());
    assert!(attempt.result.as_ref().unwrap().verified);
    assert_eq!(trace(&outer), ["begin", "value", "end"]);
    assert!(trace(&inner).is_empty());
    drop(attempt);
    assert_eq!(inner.p().refs.get(), 2);
}

#[test]
fn legacy_numeric_set_has_no_lifecycle_callbacks() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    assert!(
        p.set_raw_number(Number::Float(1.5), &mut f.guard())
            .unwrap()
            .verified
    );
    assert_eq!(trace(&f), ["value"]);
}

#[test]
fn numeric_edit_preflight_refusal_dispatches_no_begin_or_end() {
    let env = TextEnv::new();
    for scenario in 0..5 {
        let (f, p) = float_fixture(&env);
        match scenario {
            0 => f.p().readonly.set(true),
            1 => f.p().enabled.set(false),
            2 => f.p().context.set(false),
            3 => f.p().details.original.set(Some(std::ptr::null_mut())),
            _ => (),
        }
        let value = if scenario == 4 { f64::INFINITY } else { 1.5 };
        let attempt = p.edit_raw_number(Number::Float(value), &mut f.guard());
        assert_failure(&attempt, Outcome::NotApplied, NumericEditPhase::Prepared);
        assert!(trace(&f).is_empty());
        drop(attempt);
        assert_eq!(f.p().refs.get(), 1);
    }
}

#[test]
fn numeric_edit_context_loss_in_each_callback_prevents_later_dispatch() {
    let env = TextEnv::new();
    for stage in 0..3 {
        let (f, p) = float_fixture(&env);
        let field = match stage {
            0 => &f.p().edit.on_begin,
            1 => &f.p().edit.on_set,
            _ => &f.p().edit.on_end,
        };
        callback(field, |p| p.context.set(false));
        let attempt = p.edit_raw_number(Number::Float(1.5), &mut f.guard());
        let phase = [
            NumericEditPhase::BeginDispatched,
            NumericEditPhase::ValueDispatched,
            NumericEditPhase::EndDispatched,
        ][stage];
        assert_failure(&attempt, Outcome::Unknown, phase);
        let expected = [
            &["begin"][..],
            &["begin", "value"][..],
            &["begin", "value", "end"][..],
        ][stage];
        assert_eq!(trace(&f), expected);
        drop(attempt);
        assert_eq!(trace(&f), expected);
    }
}

#[test]
fn numeric_edit_original_rebinding_in_begin_or_value_never_balances_stale_receiver() {
    let env = TextEnv::new();
    for stage in 0..2 {
        let (f, p) = float_fixture(&env);
        let (replacement, _replacement) = float_fixture(&env);
        let pointer = replacement.pointer;
        callback(
            if stage == 0 {
                &f.p().edit.on_begin
            } else {
                &f.p().edit.on_set
            },
            move |p| p.details.original.set(Some(pointer)),
        );
        let attempt = p.edit_raw_number(Number::Float(1.5), &mut f.guard());
        assert_failure(
            &attempt,
            Outcome::Unknown,
            if stage == 0 {
                NumericEditPhase::BeginDispatched
            } else {
                NumericEditPhase::ValueDispatched
            },
        );
        assert!(!trace(&f).contains(&"end"));
        assert_eq!(f.p().writes.get(), stage);
    }
}

#[test]
fn numeric_edit_stable_null_controller_rebinding_prevents_set_and_end() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    let (replacement, _replacement) = float_fixture(&env);
    let pointer = replacement.pointer;
    callback(&f.p().edit.on_begin, move |p| {
        p.edit.controller.set(pointer)
    });
    let attempt = p.edit_raw_number(Number::Float(1.5), &mut f.guard());
    assert_failure(
        &attempt,
        Outcome::Unknown,
        NumericEditPhase::BeginDispatched,
    );
    assert_eq!(trace(&f), ["begin"]);
}

#[test]
fn numeric_edit_present_controller_is_retained_and_rebinding_is_unknown() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    let (controller, _controller) = float_fixture(&env);
    f.p().edit.controller.set(controller.pointer);
    callback(&f.p().edit.on_set, |p| {
        p.edit.controller.set(std::ptr::null_mut())
    });
    let attempt = p.edit_raw_number(Number::Float(1.5), &mut f.guard());
    assert_failure(
        &attempt,
        Outcome::Unknown,
        NumericEditPhase::ValueDispatched,
    );
    assert!(controller.p().refs.get() > 2);
    assert_eq!(trace(&f), ["begin", "value"]);
    drop(attempt);
    assert_eq!(controller.p().refs.get(), 2);
}

#[test]
fn numeric_edit_metadata_only_failure_closes_once_without_a_stale_value_call() {
    let env = TextEnv::new();
    for stage in 0..2 {
        for metadata in 0..3 {
            let (f, p) = float_fixture(&env);
            callback(
                if stage == 0 {
                    &f.p().edit.on_begin
                } else {
                    &f.p().edit.on_set
                },
                move |p| match metadata {
                    0 => p.readonly.set(true),
                    1 => p.enabled.set(false),
                    _ => p.max_override.set(Some(Number::Float(1.25))),
                },
            );
            let attempt = p.edit_raw_number(Number::Float(1.5), &mut f.guard());
            assert_failure(&attempt, Outcome::Unknown, NumericEditPhase::EndDispatched);
            assert_eq!(
                trace(&f),
                if stage == 0 {
                    vec!["begin", "end"]
                } else {
                    vec!["begin", "value", "end"]
                }
            );
            assert_eq!(f.p().writes.get(), stage);
            drop(attempt);
            assert_eq!(trace(&f).iter().filter(|step| **step == "end").count(), 1);
        }
    }
}

#[test]
fn numeric_edit_live_mismatch_still_ends_and_is_not_verified() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    f.p().edit.ignore_value.set(true);
    let attempt = p.edit_raw_number(Number::Float(1.5), &mut f.guard());
    assert!(!attempt.result.as_ref().unwrap().verified);
    attempt.lease.check(&mut f.guard()).unwrap();
    assert_eq!(trace(&f), ["begin", "value", "end"]);
    assert_eq!(attempt.lease.phase(), NumericEditPhase::ClosedChecked);
}

#[test]
fn numeric_edit_readback_cleanup_invalidation_is_unknown_after_end() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    callback(&f.p().edit.on_end, |p| {
        p.details.description_stop_on_drop.set(true)
    });
    let attempt = p.edit_raw_number(Number::Float(1.5), &mut f.guard());
    assert_failure(&attempt, Outcome::Unknown, NumericEditPhase::EndDispatched);
    assert_eq!(trace(&f), ["begin", "value", "end"]);
}

#[test]
fn numeric_edit_original_cycles_and_bound_refuse_before_begin() {
    let env = TextEnv::new();
    let (a, p) = float_fixture(&env);
    let (b, _b) = float_fixture(&env);
    a.p().details.original.set(Some(b.pointer));
    b.p().details.original.set(Some(a.pointer));
    let attempt = p.edit_raw_number(Number::Float(1.5), &mut a.guard());
    assert_failure(&attempt, Outcome::NotApplied, NumericEditPhase::Prepared);
    assert!(trace(&a).is_empty());
    drop(attempt);
    assert_eq!(b.p().refs.get(), 2);

    let fixtures: Vec<_> = (0..=MAX_ORIGINAL_PARAMETERS)
        .map(|_| float_fixture(&env))
        .collect();
    for pair in fixtures.windows(2) {
        pair[0].0.p().details.original.set(Some(pair[1].0.pointer));
    }
    let mut fixtures = fixtures.into_iter();
    let (first, parameter) = fixtures.next().unwrap();
    let rest: Vec<_> = fixtures.collect();
    let attempt = parameter.edit_raw_number(Number::Float(1.5), &mut first.guard());
    assert_failure(&attempt, Outcome::NotApplied, NumericEditPhase::Prepared);
    assert!(trace(&first).is_empty());
    drop(attempt);
    for (fixture, _) in &rest {
        assert_eq!(fixture.p().refs.get(), 2);
    }
}

#[test]
fn numeric_edit_publication_check_detects_later_rebinding_without_mutation() {
    let env = TextEnv::new();
    let (f, p) = float_fixture(&env);
    let (replacement, _replacement) = float_fixture(&env);
    let attempt = p.edit_raw_number(Number::Float(1.5), &mut f.guard());
    assert!(attempt.result.as_ref().unwrap().verified);
    f.p().details.original.set(Some(replacement.pointer));
    assert_eq!(
        attempt.lease.check(&mut f.guard()),
        Err(Error::MetadataChanged)
    );
    assert_eq!(trace(&f), ["begin", "value", "end"]);
}

#[test]
fn numeric_edit_last_route_guard_rebinding_cannot_authorize_the_next_callback() {
    let env = TextEnv::new();
    let (baseline, parameter) = float_fixture(&env);
    let count = Rc::new(Cell::new(0));
    let boundaries: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
    for field in [
        &baseline.p().edit.on_begin,
        &baseline.p().edit.on_set,
        &baseline.p().edit.on_end,
    ] {
        let count = count.clone();
        let boundaries = boundaries.clone();
        callback(field, move |_| boundaries.borrow_mut().push(count.get()));
    }
    let mut guard = {
        let count = count.clone();
        move || {
            count.set(count.get() + 1);
            Ok(())
        }
    };
    let attempt = parameter.edit_raw_number(Number::Float(1.5), &mut guard);
    assert!(attempt.result.as_ref().unwrap().verified);
    let mut boundaries = boundaries.borrow().clone();
    boundaries.push(count.get());
    drop(attempt);
    for (stage, boundary) in boundaries.into_iter().enumerate() {
        for change_controller in [false, true] {
            let (f, parameter) = float_fixture(&env);
            let (replacement, _replacement) = float_fixture(&env);
            let pointer = replacement.pointer;
            let mut count = 0;
            let attempt = parameter.edit_raw_number(Number::Float(1.5), &mut || {
                count += 1;
                if count == boundary {
                    if change_controller {
                        f.p().edit.controller.set(pointer);
                    } else {
                        f.p().details.original.set(Some(pointer));
                    }
                }
                Ok(())
            });
            assert_eq!(
                attempt.result.as_ref().unwrap_err().outcome,
                if stage == 0 {
                    Outcome::NotApplied
                } else {
                    Outcome::Unknown
                }
            );
            assert_eq!(
                trace(&f),
                match stage {
                    0 => vec![],
                    1 => vec!["begin"],
                    2 => vec!["begin", "value"],
                    _ => vec!["begin", "value", "end"],
                }
            );
            drop(attempt);
        }
    }
}

#[test]
fn reviewer_numeric_edit_never_sets_after_final_guard_disables_receiver() {
    let env = TextEnv::new();
    let (baseline, parameter) = float_fixture(&env);
    let count = Rc::new(Cell::new(0));
    let boundary = Rc::new(Cell::new(0));
    let c = count.clone();
    let b = boundary.clone();
    callback(&baseline.p().edit.on_set, move |_| b.set(c.get()));
    let attempt = parameter.edit_raw_number(Number::Float(1.5), &mut || {
        count.set(count.get() + 1);
        Ok(())
    });
    assert!(attempt.result.as_ref().unwrap().verified);
    drop(attempt);
    let (f, parameter) = float_fixture(&env);
    let mut current = 0;
    let attempt = parameter.edit_raw_number(Number::Float(1.5), &mut || {
        current += 1;
        if current == boundary.get() {
            f.p().readonly.set(true);
        }
        Ok(())
    });
    assert_eq!(
        f.p().writes.get(),
        0,
        "trace={:?}, phase={:?}, result={:?}",
        trace(&f),
        attempt.lease.phase(),
        attempt.result
    );
    assert_eq!(trace(&f), ["begin", "end"]);
    assert_failure(&attempt, Outcome::Unknown, NumericEditPhase::EndDispatched);
}

#[test]
fn numeric_edit_late_numeric_metadata_changes_are_checked_after_the_last_route_guard() {
    let env = TextEnv::new();
    let (baseline, parameter) = float_fixture(&env);
    let count = Rc::new(Cell::new(0));
    let boundaries = Rc::new(RefCell::new(Vec::new()));
    for field in [
        &baseline.p().edit.on_begin,
        &baseline.p().edit.on_set,
        &baseline.p().edit.on_end,
    ] {
        let count = count.clone();
        let boundaries = boundaries.clone();
        callback(field, move |_| boundaries.borrow_mut().push(count.get()));
    }
    let attempt = parameter.edit_raw_number(Number::Float(1.5), &mut || {
        count.set(count.get() + 1);
        Ok(())
    });
    assert!(attempt.result.as_ref().unwrap().verified);
    let mut boundaries = boundaries.borrow().clone();
    boundaries.push(count.get());
    drop(attempt);
    for (stage, boundary) in boundaries.into_iter().enumerate() {
        for metadata in 0..8 {
            let (f, parameter) = float_fixture(&env);
            let mut count = 0;
            let attempt = parameter.edit_raw_number(Number::Float(1.5), &mut || {
                count += 1;
                if count == boundary {
                    match metadata {
                        0 => f.p().readonly.set(true),
                        1 => f.p().enabled.set(false),
                        2 => f.p().edit.name_changed.set(true),
                        3 => f.p().edit.tag_override.set(Some(999)),
                        4 => f.p().edit.kind_override.set(Some(3)),
                        5 => *f.p().value.borrow_mut() = FakeValue::Number(Number::Integer(1)),
                        6 => f.p().max_override.set(Some(Number::Float(1.25))),
                        _ => f.p().edit.min_override.set(Some(Number::Float(1.25))),
                    }
                }
                Ok(())
            });
            assert_eq!(
                attempt.result.as_ref().unwrap_err().outcome,
                if stage == 0 {
                    Outcome::NotApplied
                } else {
                    Outcome::Unknown
                },
                "stage={stage},metadata={metadata}"
            );
            let expected = match stage {
                0 => vec![],
                1 => vec!["begin", "end"],
                _ => vec!["begin", "value", "end"],
            };
            assert_eq!(trace(&f), expected, "stage={stage},metadata={metadata}");
            assert_eq!(
                f.p().writes.get(),
                u32::from(stage >= 2),
                "stage={stage},metadata={metadata}"
            );
            drop(attempt);
            assert_eq!(trace(&f), expected);
        }
    }
}

#[test]
fn numeric_edit_unexpected_owned_final_out_is_released_before_any_safe_close() {
    let env = TextEnv::new();
    let (baseline, parameter) = float_fixture(&env);
    let count = Rc::new(Cell::new(0));
    let boundary = Rc::new(Cell::new(0));
    let c = count.clone();
    let b = boundary.clone();
    callback(&baseline.p().edit.on_set, move |_| b.set(c.get()));
    let attempt = parameter.edit_raw_number(Number::Float(1.5), &mut || {
        count.set(count.get() + 1);
        Ok(())
    });
    assert!(attempt.result.as_ref().unwrap().verified);
    drop(attempt);
    for invalidate_on_release in [false, true] {
        let (f, parameter) = float_fixture(&env);
        let native = string_new("unexpected shared object", env.stats.clone());
        if invalidate_on_release {
            *unsafe { fs(native) }.stop_on_release.borrow_mut() = Some(f.p().context.clone());
        }
        let mut count = 0;
        let context = f.p().context.clone();
        let attempt = parameter.edit_raw_number(Number::Float(1.5), &mut || {
            count += 1;
            if count == boundary.get() {
                *f.p().value.borrow_mut() = FakeValue::Object(native);
            }
            if context.get() {
                Ok(())
            } else {
                Err("OUT release invalidated context".into())
            }
        });
        assert_eq!(f.p().writes.get(), 0);
        assert_eq!(unsafe { fs(native) }.refs.get(), 1);
        assert_failure(
            &attempt,
            Outcome::Unknown,
            if invalidate_on_release {
                NumericEditPhase::BeginDispatched
            } else {
                NumericEditPhase::EndDispatched
            },
        );
        assert_eq!(
            trace(&f),
            if invalidate_on_release {
                vec!["begin"]
            } else {
                vec!["begin", "end"]
            }
        );
        drop(attempt);
        unsafe { string_release(native) };
    }
}

#[test]
fn reviewer_closed_publication_rejects_new_in_range_value_after_readback() {
    let env = TextEnv::new();
    let (_baseline, parameter) = float_fixture(&env);
    let mut count = 0;
    let attempt = parameter.edit_raw_number(Number::Float(1.5), &mut || {
        count += 1;
        Ok(())
    });
    assert!(attempt.result.as_ref().unwrap().verified);
    drop(attempt);
    let final_guard = count;
    let (f, parameter) = float_fixture(&env);
    count = 0;
    let attempt = parameter.edit_raw_number(Number::Float(1.5), &mut || {
        count += 1;
        if count == final_guard {
            *f.p().value.borrow_mut() = FakeValue::Number(Number::Float(1.0));
        }
        Ok(())
    });
    assert_eq!(trace(&f), ["begin", "value", "end"]);
    assert!(
        attempt.result.is_err(),
        "changed final value was accepted: {:?}",
        attempt.result
    );
    assert_eq!(
        attempt.result.as_ref().unwrap_err().outcome,
        Outcome::Unknown
    );
}

#[test]
fn numeric_edit_closed_receipt_retains_its_exact_observed_value() {
    let env = TextEnv::new();
    let (f, parameter) = float_fixture(&env);
    let attempt = parameter.edit_raw_number(Number::Float(1.5), &mut f.guard());
    assert!(attempt.result.as_ref().unwrap().verified);
    *f.p().value.borrow_mut() = FakeValue::Number(Number::Float(1.0));
    assert_eq!(
        attempt.lease.check(&mut f.guard()),
        Err(Error::MetadataChanged)
    );
    assert_eq!(trace(&f), ["begin", "value", "end"]);
}

fn toggle_fixture(env: &TextEnv, integer: bool) -> (Fixture, RawParameter) {
    let number = |enabled| {
        if integer {
            Number::Integer(i64::from(enabled))
        } else {
            Number::Float(f64::from(u8::from(enabled)))
        }
    };
    Fixture::new(
        env,
        0,
        FakeValue::Number(number(false)),
        number(false),
        number(true),
        None,
    )
}

#[test]
fn exact_toggle_edit_preserves_each_native_carrier_and_boolean_intent() {
    let env = TextEnv::new();
    for integer in [false, true] {
        for enabled in [false, true] {
            let (fixture, parameter) = toggle_fixture(&env, integer);
            let attempt =
                parameter.edit_raw_toggle("outerAliasName", enabled, &mut fixture.guard());
            let readback = attempt.result.as_ref().unwrap();
            assert!(readback.verified);
            assert_eq!(
                readback.after.raw.as_ref().unwrap().variant_tag,
                if integer { 1 } else { 2 }
            );
            assert_eq!(trace(&fixture), ["begin", "value", "end"]);
            assert_eq!(*fixture.p().edit.updates.borrow(), [1]);
            drop(attempt);
        }
    }
}

#[test]
fn exact_toggle_edit_refuses_changed_name_kind_or_range_before_initial_snapshot() {
    let env = TextEnv::new();
    for scenario in 0..3 {
        let (fixture, parameter) = toggle_fixture(&env, false);
        let mut guards = 0;
        let attempt = parameter.edit_raw_toggle("outerAliasName", true, &mut || {
            guards += 1;
            if guards == 1 {
                match scenario {
                    0 => fixture.p().edit.name_changed.set(true),
                    1 => fixture.p().edit.kind_override.set(Some(2)),
                    2 => fixture.p().edit.min_override.set(Some(Number::Float(-1.0))),
                    _ => unreachable!(),
                }
            }
            Ok(())
        });
        assert_failure(&attempt, Outcome::NotApplied, NumericEditPhase::Prepared);
        assert!(trace(&fixture).is_empty());
        assert_eq!(fixture.p().writes.get(), 0);
        assert!(guards > 1);
    }
}

#[test]
fn exact_toggle_edit_refuses_noncanonical_values_carriers_and_wrong_name() {
    let env = TextEnv::new();
    for scenario in 0..5 {
        let (fixture, parameter) = toggle_fixture(&env, false);
        match scenario {
            0 => *fixture.p().value.borrow_mut() = FakeValue::Number(Number::Float(-0.0)),
            1 => *fixture.p().value.borrow_mut() = FakeValue::Number(Number::Float(0.5)),
            2 => fixture.p().edit.min_override.set(Some(Number::Integer(0))),
            3 => fixture.p().max_override.set(Some(Number::Float(2.0))),
            4 => (),
            _ => unreachable!(),
        }
        let attempt = parameter.edit_raw_toggle(
            if scenario == 4 {
                "autofill"
            } else {
                "outerAliasName"
            },
            true,
            &mut fixture.guard(),
        );
        assert_failure(&attempt, Outcome::NotApplied, NumericEditPhase::Prepared);
        assert!(trace(&fixture).is_empty());
        assert_eq!(fixture.p().writes.get(), 0);
    }
}

#[test]
fn exact_toggle_metadata_changes_after_begin_balance_end_without_a_value_write() {
    let env = TextEnv::new();
    for scenario in 0..4 {
        let (fixture, parameter) = toggle_fixture(&env, false);
        callback(
            &fixture.p().edit.on_begin,
            move |parameter| match scenario {
                0 => parameter.enabled.set(false),
                1 => parameter.edit.name_changed.set(true),
                2 => parameter.edit.kind_override.set(Some(2)),
                3 => parameter.edit.min_override.set(Some(Number::Float(-1.0))),
                _ => unreachable!(),
            },
        );
        let attempt = parameter.edit_raw_toggle("outerAliasName", true, &mut fixture.guard());
        assert_failure(&attempt, Outcome::Unknown, NumericEditPhase::EndDispatched);
        assert_eq!(trace(&fixture), ["begin", "end"]);
        assert_eq!(fixture.p().writes.get(), 0);
        drop(attempt);
        assert_eq!(trace(&fixture), ["begin", "end"]);
    }
}

#[test]
fn exact_toggle_attachment_loss_after_begin_never_sends_an_end() {
    let env = TextEnv::new();
    let (fixture, parameter) = toggle_fixture(&env, true);
    callback(&fixture.p().edit.on_begin, |parameter| {
        parameter.context.set(false)
    });
    let attempt = parameter.edit_raw_toggle("outerAliasName", true, &mut fixture.guard());
    assert_failure(
        &attempt,
        Outcome::Unknown,
        NumericEditPhase::BeginDispatched,
    );
    assert_eq!(trace(&fixture), ["begin"]);
    assert_eq!(fixture.p().writes.get(), 0);
    drop(attempt);
    assert_eq!(trace(&fixture), ["begin"]);
}
