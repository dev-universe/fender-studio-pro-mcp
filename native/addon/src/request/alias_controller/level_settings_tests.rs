use super::*;

fn value(n: i64) -> Option<parameters::NativeValue> {
    Some(parameters::NativeValue {
        variant_tag: 1,
        value: parameters::Value::Integer(n),
    })
}
fn description(s: Setting) -> parameters::Description {
    parameters::Description {
        name: s.name.into(),
        tag: s.tag,
        kind: s.kind,
        enabled: true,
        read_only: false,
        raw: value(0),
        min: value(0),
        max: value(s.max),
        default_value: value(0),
        normalized: parameters::Normalized::NotApplicable,
        display: "native observed label".into(),
        display_parser: parameters::DisplayParser::Unsupported,
        support: parameters::Support {
            raw_number: true,
            normalized: false,
            display_text: false,
        },
    }
}
#[test]
fn only_six_registered_settings_have_access() {
    assert_eq!(SETTINGS.len(), 6);
    for name in [
        "mode",
        "rmsLength",
        "holdLength",
        "vuHold",
        "loudnessInLUFS",
        "loudnessEBU18",
    ] {
        assert!(Settings::registered(name).is_ok());
    }
    for name in [
        "levelHold1",
        "correlation",
        "resetLoudness",
        "holdLengthMenu",
        "modeoptions",
        "Mode",
        "OutLevels/mode",
        "",
    ] {
        assert!(Settings::registered(name).is_err());
    }
}
#[test]
fn every_setting_rejects_name_tag_kind_carrier_and_bounds_drift() {
    for s in SETTINGS {
        let good = description(s);
        assert!(s.validate(&good).is_ok());
        for field in 0..7 {
            let mut d = good.clone();
            match field {
                0 => d.name = "levelHold1".into(),
                1 => d.tag += 1,
                2 => d.kind = parameters::Kind::Command,
                3 => d.raw.as_mut().unwrap().variant_tag = 2,
                4 => d.min = value(-1),
                5 => d.max = value(s.max + 1),
                _ => d.raw = value(s.max + 1),
            }
            assert!(s.validate(&d).is_err());
        }
        for n in 0..=s.max {
            assert_eq!(s.raw(&json!(n)).unwrap(), n);
            assert_eq!(s.raw(&json!(n as f64)).unwrap(), n);
        }
        for bad in [
            json!(-1),
            json!(s.max + 1),
            json!(0.5),
            json!("0"),
            json!(true),
            Value::Null,
        ] {
            assert!(s.raw(&bad).is_err());
        }
    }
}
#[test]
fn source_requires_a_bound_instance_and_rejects_undeclared_operations() {
    let parse = |v| {
        let r: Request = serde_json::from_value(v).unwrap();
        r.validate()
    };
    let id = "61D64D66-2A46-4BEE-A623-EC5CEB898313";
    for op in ["parameter", "choices", "format_values", "set_parameter"] {
        let mut r =
            json!({"operation":op,"source":"level_meter","param":"mode","expected_instance":id});
        if op == "format_values" {
            r["values"] = json!([0, 1]);
        }
        if op == "set_parameter" {
            r["mode"] = json!("raw");
            r["value"] = json!(1);
        }
        assert!(parse(r.clone()).is_ok());
        r["param"] = json!("resetLoudness");
        assert!(parse(r.clone()).is_err());
        r["param"] = json!("mode");
        r.as_object_mut().unwrap().remove("expected_instance");
        assert!(parse(r).is_err());
    }
    assert!(parse(json!({"operation":"parameter_details","source":"level_meter","param":"mode","expected_instance":id})).is_err());
}
