use super::*;

pub(super) fn inspect(
    ctx: &Context<'_>,
    provider: &Provider,
    param: &str,
) -> Result<Value, String> {
    let controller = provider
        .scalar()
        .ok_or("parameter details require a scalar controller")?;
    let mut guard = || {
        controller.check()?;
        ctx.check()
    };
    let chain = observe(controller, param, ctx.api, &mut guard)?;
    let coverage = match provider {
        Provider::DspValue(_) => "direct_device_named_parameter",
        _ => controller.coverage(),
    };
    Ok(report(param, &chain, coverage))
}

fn observe<G: FnMut() -> Result<(), String>>(
    controller: &Controller,
    name: &str,
    api: &Api,
    guard: &mut G,
) -> Result<parameters::OriginalChain, String> {
    guard()?;
    // Controller::find retains the borrowed parameter before any guard reentry.
    let parameter = controller.find(name, api);
    let result = match (guard(), parameter.as_ref()) {
        (Ok(()), Ok(parameter)) => parameter
            .original_chain(guard)
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

fn metadata(details: &parameters::ParameterDetails) -> Value {
    json!({
        "precision":details.precision,
        "storable":details.storable,
        "mutable":details.mutable,
        "store_list_value":details.store_list_value,
        "read_only":details.read_only,
        "can_undo":details.can_undo,
        "list_interface":details.list_interface,
    })
}

fn snapshot(parameter: &parameters::OriginalParameter) -> Value {
    let mut value = description(&parameter.description);
    value["native_min"] = typed(parameter.description.min.as_ref());
    value["native_max"] = typed(parameter.description.max.as_ref());
    value["native_default"] = typed(parameter.description.default_value.as_ref());
    json!({"description":value,"details":metadata(&parameter.details)})
}

fn report(param: &str, chain: &parameters::OriginalChain, coverage: &str) -> Value {
    let records: Vec<_> = chain.parameters.iter().map(snapshot).collect();
    let end = match chain.end {
        parameters::OriginalChainEnd::SelfReference => json!({"kind":"self_reference"}),
        parameters::OriginalChainEnd::MissingOriginal => json!({"kind":"missing_original"}),
        parameters::OriginalChainEnd::Cycle { index } => json!({"kind":"cycle","index":index}),
        parameters::OriginalChainEnd::Limit => json!({"kind":"limit"}),
    };
    json!({
        "param":param,
        "parameter":records[0]["description"],
        "details":records[0]["details"],
        "original_chain":{
            "parameters":records,
            "end":end,
            "observed_count":chain.parameters.len(),
            "max_parameters":parameters::MAX_ORIGINAL_PARAMETERS,
        },
        "coverage":coverage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn original(value: parameters::Value, min: parameters::Value) -> parameters::OriginalParameter {
        parameters::OriginalParameter {
            description: parameters::Description {
                name: "model".into(),
                tag: 42,
                kind: parameters::Kind::List,
                enabled: true,
                read_only: false,
                raw: Some(parameters::NativeValue {
                    variant_tag: 2,
                    value,
                }),
                min: Some(parameters::NativeValue {
                    variant_tag: 1,
                    value: min,
                }),
                max: None,
                default_value: None,
                normalized: parameters::Normalized::Finite(0.5),
                display: "Observed model".into(),
                display_parser: parameters::DisplayParser::Unsupported,
                support: parameters::Support {
                    raw_number: true,
                    normalized: true,
                    display_text: false,
                },
            },
            details: parameters::ParameterDetails {
                precision: 10_000,
                storable: true,
                mutable: true,
                store_list_value: false,
                read_only: false,
                can_undo: true,
                list_interface: false,
            },
        }
    }

    #[test]
    fn report_preserves_current_and_original_typed_values_without_inferring_choices() {
        let mut underlying = original(parameters::Value::Integer(1), parameters::Value::Integer(0));
        underlying.description.raw.as_mut().unwrap().variant_tag = 1;
        underlying.description.max = Some(parameters::NativeValue {
            variant_tag: 1,
            value: parameters::Value::Integer(3),
        });
        underlying.details.precision = 3;
        underlying.details.list_interface = true;
        let facade = original(
            parameters::Value::Float(-0.0),
            parameters::Value::Integer(i64::MIN),
        );
        let chain = parameters::OriginalChain {
            parameters: vec![facade, underlying],
            end: parameters::OriginalChainEnd::SelfReference,
        };
        let value = report("opt.ampmodel", &chain, "direct_device_named_parameter");
        assert_eq!(
            value["parameter"]["native_value"]["data"]["bits_hex"],
            "8000000000000000"
        );
        assert_eq!(
            value["parameter"]["native_min"]["data"]["exact_decimal"],
            "-9223372036854775808"
        );
        assert_eq!(value["parameter"]["native_max"], Value::Null);
        assert_eq!(value["details"]["precision"], 10_000);
        assert_eq!(value["details"]["list_interface"], false);
        assert_eq!(value["original_chain"]["observed_count"], 2);
        assert_eq!(value["original_chain"]["max_parameters"], 8);
        assert_eq!(
            value["original_chain"]["parameters"][1]["description"]["native_value"]["data"]["value"],
            1
        );
        assert_eq!(
            value["original_chain"]["parameters"][1]["details"]["list_interface"],
            true
        );
        assert_eq!(value["coverage"], "direct_device_named_parameter");
        assert!(value.get("reported_count").is_none());
        assert!(value.get("choices").is_none());
    }

    #[test]
    fn report_keeps_each_chain_endpoint_explicit() {
        for (end, expected) in [
            (
                parameters::OriginalChainEnd::SelfReference,
                json!({"kind":"self_reference"}),
            ),
            (
                parameters::OriginalChainEnd::MissingOriginal,
                json!({"kind":"missing_original"}),
            ),
            (
                parameters::OriginalChainEnd::Cycle { index: 0 },
                json!({"kind":"cycle","index":0}),
            ),
            (parameters::OriginalChainEnd::Limit, json!({"kind":"limit"})),
        ] {
            let chain = parameters::OriginalChain {
                parameters: vec![original(
                    parameters::Value::Integer(1),
                    parameters::Value::Integer(0),
                )],
                end,
            };
            let value = report("model", &chain, "resolved_controller_provider");
            assert_eq!(value["original_chain"]["end"], expected);
            assert!(value["original_chain"].get("complete").is_none());
        }
    }

    #[test]
    fn parameter_details_requires_an_explicit_source_and_expected_instance() {
        let valid = json!({
            "operation":"parameter_details",
            "source":"dsp",
            "param":"opt.ampmodel",
            "expected_instance":"61D64D66-2A46-4BEE-A623-EC5CEB898313",
        });
        for source in ["dsp", "controller"] {
            let mut value = valid.clone();
            value["source"] = json!(source);
            assert!(
                serde_json::from_value::<Request>(value)
                    .unwrap()
                    .validate()
                    .is_ok()
            );
        }
        let mut invalids = Vec::new();
        for field in ["source", "expected_instance", "param"] {
            let mut value = valid.clone();
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
            ("include_details", json!(true)),
            ("value", json!(1)),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            invalids.push(invalid);
        }
        for invalid in invalids {
            assert!(
                serde_json::from_value::<Request>(invalid)
                    .and_then(|request| request.validate().map_err(serde::de::Error::custom))
                    .is_err()
            );
        }
    }

    #[repr(C)]
    struct Fake {
        table: *const *const c_void,
        refs: Cell<u32>,
        parameter: *mut c_void,
        finds: Cell<u32>,
        stop: Cell<bool>,
    }
    unsafe extern "C" fn retain(pointer: *mut c_void) -> u32 {
        let fake = unsafe { &*pointer.cast::<Fake>() };
        fake.refs.set(fake.refs.get() + 1);
        fake.refs.get()
    }
    unsafe extern "C" fn release(pointer: *mut c_void) -> u32 {
        let fake = unsafe { &*pointer.cast::<Fake>() };
        fake.stop.set(true);
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
    fn stale_context_prevents_lookup_and_lookup_reentry_still_releases_parameter() {
        let mut table = [ptr::null(); 6];
        table[1] = retain as *const c_void;
        table[2] = release as *const c_void;
        table[5] = find as *const c_void;
        let mut parameter = Fake {
            table: table.as_ptr(),
            refs: Cell::new(1),
            parameter: ptr::null_mut(),
            finds: Cell::new(0),
            stop: Cell::new(false),
        };
        let mut raw_controller = Fake {
            table: table.as_ptr(),
            refs: Cell::new(1),
            parameter: ptr::addr_of_mut!(parameter).cast(),
            finds: Cell::new(0),
            stop: Cell::new(false),
        };
        let controller = Controller {
            handle: Owned(ptr::addr_of_mut!(raw_controller).cast()),
            lease: None,
        };
        assert!(observe(&controller, "model", &api(), &mut || Err("stale".into())).is_err());
        assert_eq!(raw_controller.finds.get(), 0);
        let mut guard = || {
            if raw_controller.finds.get() == 0 {
                Ok(())
            } else {
                Err("lookup changed context".into())
            }
        };
        assert!(observe(&controller, "model", &api(), &mut guard).is_err());
        assert_eq!(raw_controller.finds.get(), 1);
        assert_eq!(parameter.refs.get(), 1);
        assert!(parameter.stop.get());
        drop(controller);
        assert_eq!(raw_controller.refs.get(), 0);
    }
}
