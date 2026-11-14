use super::*;

pub(super) fn inspect(
    ctx: &Context<'_>,
    provider: &Provider,
    param: &str,
    values: &[f64],
) -> Result<Value, String> {
    let controller = provider
        .scalar()
        .ok_or("format_values requires a scalar controller")?;
    let mut guard = || {
        controller.check()?;
        ctx.check()
    };
    let values: Vec<_> = values
        .iter()
        .copied()
        .map(parameters::Number::Float)
        .collect();
    let formatted = observe(controller, param, &values, ctx.api, &mut guard)?;
    let coverage = match provider {
        Provider::DspValue(_) => "direct_device_named_parameter",
        _ => controller.coverage(),
    };
    Ok(report(param, &formatted, coverage))
}

fn observe<G: FnMut() -> Result<(), String>>(
    controller: &Controller,
    name: &str,
    values: &[parameters::Number],
    api: &Api,
    guard: &mut G,
) -> Result<parameters::FormattedValues, String> {
    guard()?;
    let parameter = controller.find(name, api);
    let result = match (guard(), parameter.as_ref()) {
        (Ok(()), Ok(parameter)) => parameter
            .format_values(values, guard)
            .map_err(|error| error.to_string()),
        (Err(error), _) => Err(error),
        (_, Err(error)) => Err(error.clone()),
    };
    let before_release = guard();
    drop(parameter);
    let after_release = guard();
    before_release?;
    after_release?;
    result
}

fn snapshot(description: &parameters::Description) -> Value {
    let mut value = super::description(description);
    value["native_min"] = typed(description.min.as_ref());
    value["native_max"] = typed(description.max.as_ref());
    value["native_default"] = typed(description.default_value.as_ref());
    value
}

pub(super) fn report(
    param: &str,
    formatted: &parameters::FormattedValues,
    coverage: &str,
) -> Value {
    let values: Vec<_> = formatted
        .values
        .iter()
        .map(|item| {
            let requested = match item.requested {
                parameters::Number::Integer(value) => parameters::NativeValue {
                    variant_tag: 1,
                    value: parameters::Value::Integer(value),
                },
                parameters::Number::Float(value) => parameters::NativeValue {
                    variant_tag: 2,
                    value: parameters::Value::Float(value),
                },
            };
            json!({
                "requested_value":scalar(Some(&requested)),
                "requested_native_value":typed(Some(&requested)),
                "native_value":typed(Some(&item.native_value)),
                "text":item.label,
            })
        })
        .collect();
    json!({
        "param":param,
        "values":values,
        "before":snapshot(&formatted.before),
        "after":snapshot(&formatted.after),
        "current_unchanged":formatted.current_unchanged,
        "coverage":coverage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn request() -> Value {
        json!({"operation":"format_values","source":"dsp","param":"opt.ampmodel","values":[0.5,2,0.5],"expected_instance":"61D64D66-2A46-4BEE-A623-EC5CEB898313"})
    }
    fn parse(value: Value) -> Result<Request, String> {
        let request: Request = serde_json::from_value(value).map_err(|error| error.to_string())?;
        request.validate()?;
        Ok(request)
    }

    #[test]
    fn finite_format_requests_require_explicit_target_source_and_bounded_numbers() {
        for source in ["dsp", "controller"] {
            let mut value = request();
            value["source"] = json!(source);
            assert!(parse(value).is_ok());
        }
        let mut boundary = request();
        boundary["values"] = json!(vec![MAX_SAFE_INTEGER; 32]);
        assert!(parse(boundary).is_ok());
        let mut invalids = Vec::new();
        for field in ["param", "source", "values", "expected_instance"] {
            let mut value = request();
            value.as_object_mut().unwrap().remove(field);
            invalids.push(value);
        }
        for (field, value) in [
            ("source", json!("auto")),
            ("source", json!("alias")),
            ("param", json!("")),
            ("param", json!("x".repeat(513))),
            ("param", json!("bad\0name")),
            ("expected_instance", json!(ZERO_UID)),
            ("values", json!([])),
            ("values", json!(vec![1; 33])),
            ("values", json!([null])),
            ("values", json!([true])),
            ("values", json!(["1"])),
            ("values", json!([[1]])),
            ("values", json!([9007199254740992u64])),
            ("count", json!(6)),
            ("precision", json!(5)),
            ("mode", json!("text")),
        ] {
            let mut invalid = request();
            invalid[field] = value;
            invalids.push(invalid);
        }
        for invalid in invalids {
            assert!(parse(invalid).is_err());
        }
        for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let invalid = Request::FormatValues {
                param: "model".into(),
                source: Source::Dsp,
                values: vec![number],
                expected_instance: Some("61D64D66-2A46-4BEE-A623-EC5CEB898313".into()),
            };
            assert!(invalid.validate().is_err());
        }
    }

    fn description() -> parameters::Description {
        parameters::Description {
            name: "model".into(),
            tag: 1,
            kind: parameters::Kind::List,
            enabled: true,
            read_only: false,
            raw: Some(parameters::NativeValue {
                variant_tag: 2,
                value: parameters::Value::Float(-0.0),
            }),
            min: None,
            max: None,
            default_value: None,
            normalized: parameters::Normalized::Finite(-0.0),
            display: "Current".into(),
            display_parser: parameters::DisplayParser::Unsupported,
            support: parameters::Support {
                raw_number: true,
                normalized: true,
                display_text: false,
            },
        }
    }

    #[test]
    fn reports_pass_order_exact_carriers_and_unchanged_fact_without_choice_inference() {
        let result = parameters::FormattedValues {
            values: vec![
                parameters::FormattedValue {
                    requested: parameters::Number::Float(-0.0),
                    native_value: parameters::NativeValue {
                        variant_tag: 2,
                        value: parameters::Value::Float(-0.0),
                    },
                    label: "Observed zero".into(),
                },
                parameters::FormattedValue {
                    requested: parameters::Number::Float(2.0),
                    native_value: parameters::NativeValue {
                        variant_tag: 1,
                        value: parameters::Value::Integer(2),
                    },
                    label: "Observed two".into(),
                },
                parameters::FormattedValue {
                    requested: parameters::Number::Float(-0.0),
                    native_value: parameters::NativeValue {
                        variant_tag: 2,
                        value: parameters::Value::Float(-0.0),
                    },
                    label: "Observed zero".into(),
                },
            ],
            before: description(),
            after: description(),
            current_unchanged: true,
        };
        let value = report("model", &result, "direct_device_named_parameter");
        assert_eq!(value["values"].as_array().unwrap().len(), 3);
        assert_eq!(value["values"][0]["text"], "Observed zero");
        assert_eq!(
            value["values"][0]["requested_native_value"]["data"]["bits_hex"],
            "8000000000000000"
        );
        assert_eq!(value["values"][1]["native_value"]["variant_tag"], 1);
        assert_eq!(value["values"][1]["native_value"]["data"]["value"], 2);
        assert_eq!(value["values"][2], value["values"][0]);
        assert_eq!(
            value["before"]["native_value"]["data"]["bits_hex"],
            "8000000000000000"
        );
        assert_eq!(value["current_unchanged"], true);
        assert!(value.get("reported_count").is_none());
        assert!(value.get("choices").is_none());
        assert!(value.get("selected").is_none());
    }

    #[repr(C)]
    struct Fake {
        table: *const *const c_void,
        refs: Cell<u32>,
        parameter: *mut c_void,
        finds: Cell<u32>,
    }
    unsafe extern "C" fn retain(pointer: *mut c_void) -> u32 {
        let fake = unsafe { &*pointer.cast::<Fake>() };
        fake.refs.set(fake.refs.get() + 1);
        fake.refs.get()
    }
    unsafe extern "C" fn release(pointer: *mut c_void) -> u32 {
        let fake = unsafe { &*pointer.cast::<Fake>() };
        fake.refs.set(fake.refs.get() - 1);
        fake.refs.get()
    }
    unsafe extern "C" fn find(pointer: *mut c_void, _: *const PlainCString) -> *mut c_void {
        let fake = unsafe { &*pointer.cast::<Fake>() };
        fake.finds.set(fake.finds.get() + 1);
        fake.parameter
    }
    unsafe extern "C" fn empty() -> *mut c_void {
        ptr::null_mut()
    }
    unsafe extern "C" fn text(_: *const c_char) -> *const PlainCString {
        ptr::null()
    }
    unsafe extern "C" fn thread() -> i64 {
        0
    }
    fn api() -> Api {
        Api {
            gui: empty,
            objects: empty,
            main_thread: empty,
            thread_self: thread,
            cstring: text,
            empty,
            desktop: empty,
            scripting: empty,
            plugin_manager: empty,
            file_system: empty,
        }
    }

    #[test]
    fn stale_guard_prevents_find_and_reentrant_find_still_drops_the_parameter() {
        let mut table = [ptr::null(); 6];
        table[1] = retain as *const c_void;
        table[2] = release as *const c_void;
        table[5] = find as *const c_void;
        let mut parameter = Fake {
            table: table.as_ptr(),
            refs: Cell::new(1),
            parameter: ptr::null_mut(),
            finds: Cell::new(0),
        };
        let mut raw = Fake {
            table: table.as_ptr(),
            refs: Cell::new(1),
            parameter: ptr::addr_of_mut!(parameter).cast(),
            finds: Cell::new(0),
        };
        let controller = Controller {
            handle: Owned(ptr::addr_of_mut!(raw).cast()),
            lease: None,
        };
        assert!(
            observe(
                &controller,
                "model",
                &[parameters::Number::Float(1.0)],
                &api(),
                &mut || Err("stale".into())
            )
            .is_err()
        );
        assert_eq!(raw.finds.get(), 0);
        let mut guard = || {
            if raw.finds.get() == 0 {
                Ok(())
            } else {
                Err("context replaced".into())
            }
        };
        assert!(
            observe(
                &controller,
                "model",
                &[parameters::Number::Float(1.0)],
                &api(),
                &mut guard
            )
            .is_err()
        );
        assert_eq!(raw.finds.get(), 1);
        assert_eq!(parameter.refs.get(), 1);
        drop(controller);
        assert_eq!(raw.refs.get(), 0);
    }
}
