use serde_json::{Map, Value, json};

use crate::{bridge::WireResponse, model::HostContext};

pub(super) const SETTLE_MILLIS: u64 = 150;

pub(super) fn unavailable(reason: &str) -> Value {
    json!({"status": "unavailable", "reason": reason})
}

pub(super) fn observe(
    write_args: &Value,
    context: &HostContext,
    write: &Value,
    read: impl FnOnce(Value) -> Option<WireResponse>,
) -> Value {
    let Some(name) = write_args["param"].as_str() else {
        return unavailable("invalid_write_receipt");
    };
    let (Some(expected_instance), Some(actual_instance)) = (
        write_args["expected_instance"].as_str(),
        write["instance"]["object_uid"].as_str(),
    ) else {
        return unavailable("invalid_write_receipt");
    };
    let after = &write["after"];
    if write["native_verified"] != true
        || write["source"] != write_args["source"]
        || write["param"] != name
        || write["plugin"].as_str().is_none_or(str::is_empty)
        || write["instance"]["class_uid"]
            .as_str()
            .is_none_or(str::is_empty)
        || !actual_instance.eq_ignore_ascii_case(expected_instance)
        || !has_value(after, name)
    {
        return unavailable("invalid_write_receipt");
    }

    let mut query = Map::new();
    for key in [
        "channel",
        "slot",
        "kind",
        "track",
        "plugin",
        "param",
        "source",
        "expected_instance",
    ] {
        if let Some(value) = write_args.get(key) {
            query.insert(key.to_owned(), value.clone());
        }
    }
    let Some(response) = read(Value::Object(query)) else {
        return unavailable("read_failed");
    };
    if response.session != context.session || response.document != context.document {
        return unavailable("context_changed");
    }
    let result = &response.result;
    if !response.ok || result["ok"] != true || !result["error"].is_null() {
        return unavailable("read_failed");
    }
    if ["instance", "plugin", "source", "param"]
        .iter()
        .any(|&field| result.get(field) != write.get(field))
    {
        return unavailable("target_changed");
    }
    let parameter = &result["parameter"];
    if !has_value(parameter, name)
        || parameter["tag"] != after["tag"]
        || parameter["native_kind"] != after["native_kind"]
    {
        return unavailable("parameter_unavailable");
    }
    let same = parameter["native_value"] == after["native_value"]
        && parameter.get("native_normalized") == after.get("native_normalized");
    json!({
        "status": if same { "matched" } else { "mismatch" },
        "settle_millis": SETTLE_MILLIS,
        "parameter": parameter,
    })
}

fn has_value(row: &Value, name: &str) -> bool {
    let value = &row["native_value"];
    row["name"] == name
        && row["tag"]
            .as_i64()
            .is_some_and(|tag| i32::try_from(tag).is_ok())
        && row["native_kind"]
            .as_str()
            .is_some_and(|kind| !kind.is_empty())
        && matches!(
            value["data"]["kind"].as_str(),
            Some("integer" | "float" | "text")
        )
        && super::gui_default_scalar(value, &row["value"])
        && match row["native_normalized"]["kind"].as_str() {
            Some("not_applicable") => true,
            Some("float32") => row["native_normalized"]["bits_hex"]
                .as_str()
                .is_some_and(|hex| {
                    hex.len() == 8
                        && u32::from_str_radix(hex, 16)
                            .is_ok_and(|bits| f32::from_bits(bits).is_finite())
                }),
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::Operation;

    const INSTANCE: &str = "D40823BB-2810-425E-ACEE-11D1D7185024";

    fn args() -> Value {
        json!({"plugin":"Pro-G","channel":"Scratch","slot":0,"param":"vstp1073741828",
            "source":"dsp","expected_instance":INSTANCE,"normalized":0.035})
    }

    fn context() -> HostContext {
        HostContext {
            session: "session".to_owned(),
            document: "session:1".to_owned(),
            open: true,
            title: None,
            file_url: None,
            protocol: crate::bridge::PROTOCOL,
        }
    }

    fn parameter(value: f64, norm: f32) -> Value {
        json!({"name":"vstp1073741828","tag":1073741829,"native_kind":"Float","value":value,
            "native_value":{"variant_tag":2,"data":{"kind":"float","value":value,"bits_hex":format!("{:016X}",value.to_bits())}},
            "native_normalized":{"kind":"float32","bits_hex":format!("{:08X}",norm.to_bits())}})
    }

    fn immediate() -> Value {
        json!({"plugin":"Pro-G","slot":0,"channel":"Scratch","param":"vstp1073741828","source":"dsp",
            "native_verified":true,"instance":{"object_uid":INSTANCE,"class_uid":"9D1DCCF7-918A-436F-A3CD-D7115D8B193B"},
            "after":parameter(350.0000014901161,0.035)})
    }

    fn response(write: &Value, parameter: Value) -> WireResponse {
        WireResponse {
            protocol: crate::bridge::PROTOCOL,
            id: "read".to_owned(),
            session: "session".to_owned(),
            document: "session:1".to_owned(),
            ok: true,
            result: json!({"ok":true,"plugin":"Pro-G","instance":write["instance"],
                "source":write["source"],"param":write["param"],"parameter":parameter}),
            error: None,
            code: None,
        }
    }

    #[test]
    fn checks_the_same_native_target_without_forwarding_a_write() {
        let write = immediate();
        let report = observe(&args(), &context(), &write, |query| {
            assert_eq!(
                query,
                json!({"plugin":"Pro-G","channel":"Scratch","slot":0,
                    "source":"dsp","expected_instance":INSTANCE,"param":"vstp1073741828"})
            );
            Some(response(&write, write["after"].clone()))
        });
        assert_eq!(report["status"], "matched");
        assert_eq!(report["parameter"], write["after"]);
        assert_eq!(report["settle_millis"], SETTLE_MILLIS);
    }

    #[test]
    fn transient_legacy_write_reports_the_separately_observed_zero() {
        let write = immediate();
        let zero = parameter(0.0, 0.0);
        let report = observe(&args(), &context(), &write, |_| {
            Some(response(&write, zero.clone()))
        });
        assert_eq!(report["status"], "mismatch");
        assert_eq!(report["parameter"], zero);
    }

    #[test]
    fn discards_readback_on_document_or_instance_change() {
        let write = immediate();
        let result = observe(&args(), &context(), &write, |_| {
            let mut read = response(&write, write["after"].clone());
            read.document = "session:2".to_owned();
            Some(read)
        });
        assert_eq!(result, unavailable("context_changed"));
        let result = observe(&args(), &context(), &write, |_| {
            let mut read = response(&write, write["after"].clone());
            read.result["instance"]["object_uid"] = json!("different");
            Some(read)
        });
        assert_eq!(result, unavailable("target_changed"));
    }

    #[test]
    fn discards_other_sources_and_invalid_parameter_evidence() {
        let write = immediate();
        for (pointer, invalid, reason) in [
            ("/source", json!("controller"), "target_changed"),
            ("/parameter/tag", json!(7), "parameter_unavailable"),
            (
                "/parameter/native_value/data/bits_hex",
                json!("0000000000000000"),
                "parameter_unavailable",
            ),
        ] {
            let result = observe(&args(), &context(), &write, |_| {
                let mut read = response(&write, write["after"].clone());
                *read.result.pointer_mut(pointer).unwrap() = invalid;
                Some(read)
            });
            assert_eq!(result, unavailable(reason));
        }
    }

    #[test]
    fn requires_native_source_and_instance_and_keeps_the_old_bridge_payload() {
        let mut validated = args();
        validated["readback"] = json!(true);
        let operation = Operation::from_parts("plugin_parameter_set", validated.clone()).unwrap();
        assert!(operation.validate().is_ok());
        assert_eq!(operation.wire(), ("setPluginParam".to_owned(), args()));
        let mut disabled = args();
        disabled["readback"] = json!(false);
        let operation = Operation::from_parts("plugin_parameter_set", disabled).unwrap();
        assert_eq!(operation.wire(), ("setPluginParam".to_owned(), args()));
        for key in ["source", "expected_instance"] {
            let mut invalid = validated.clone();
            invalid.as_object_mut().unwrap().remove(key);
            assert!(
                Operation::from_parts("plugin_parameter_set", invalid)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
    }
}
