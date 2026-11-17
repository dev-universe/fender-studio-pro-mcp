use super::*;

pub(super) fn synthetic(model: &str) -> String {
    format!(
        "<MusicEffectPreset cid=\"{{{CLASS}}}\"><Attributes x:id=\"ComponentData\"><Attributes x:id=\"chorder\" transpose=\"2\" loKey=\"9\" hiKey=\"120\" autofill=\"0\" filterOutsideRange=\"1\"/></Attributes><Attributes x:id=\"AlgorithmData\"><Attributes x:id=\"model\">{model}</Attributes></Attributes></MusicEffectPreset>"
    )
}
fn input(pitch: u8, outputs: &[u8]) -> String {
    format!(
        "<Attributes inPitch=\"{pitch}\">{}</Attributes>",
        outputs
            .iter()
            .map(|output| format!("<Attributes outPitch=\"{output}\"/>"))
            .collect::<String>()
    )
}
fn instance() -> Instance {
    Instance {
        object_uid: uid(&SERVICE_CID),
        class_uid: CLASS.into(),
        native_class: "SyntheticChorder".into(),
        object_id: "test".into(),
    }
}

#[test]
fn complete_model_pages_include_empty_keys_and_sort_outputs() {
    let bytes = synthetic(&(input(127, &[60]) + &input(0, &[127, 0]))).into_bytes();
    let model = decode(&bytes).unwrap();
    let mut all = Vec::new();
    for offset in (0..128).step_by(16) {
        let page = model.page(offset, 16, &bytes, &instance());
        assert_eq!(page["offset"], offset);
        assert_eq!(page["count"], 16);
        assert_eq!(
            page["next"],
            if offset == 112 {
                Value::Null
            } else {
                json!(offset + 16)
            }
        );
        assert_eq!(page["mutation_capability"], false);
        assert_eq!(page["persistence_verified"], false);
        assert_eq!(page["audio_verified"], false);
        assert!(page.get("bytes").is_none());
        assert!(page.get("token").is_none());
        all.extend(page["chords"].as_array().unwrap().iter().cloned());
    }
    assert_eq!(all.len(), 128);
    assert_eq!(all[0], json!({"input_pitch":0,"output_pitches":[0,127]}));
    assert_eq!(all[1], json!({"input_pitch":1,"output_pitches":[]}));
    assert_eq!(all[127], json!({"input_pitch":127,"output_pitches":[60]}));
    let tail = model.page(127, 16, &bytes, &instance());
    assert_eq!(tail["count"], 1);
    assert_eq!(tail["next"], Value::Null);
}

#[test]
fn semantic_hash_ignores_xml_order_and_retains_every_midi_boundary() {
    let a = synthetic(&(input(127, &[60]) + &input(0, &[127, 0]))).into_bytes();
    let b = synthetic(&(input(0, &[0, 127]) + &input(127, &[60]))).into_bytes();
    let first = decode(&a).unwrap().page(0, 1, &a, &instance());
    let second = decode(&b).unwrap().page(0, 1, &b, &instance());
    assert_eq!(first["model_sha256"], second["model_sha256"]);
    assert_ne!(first["capture_sha256"], second["capture_sha256"]);
    for changed in [input(0, &[127]), input(127, &[0]), input(0, &[0, 126])] {
        assert_ne!(
            decode(synthetic(&changed).as_bytes()).unwrap().digest(),
            first["model_sha256"]
        );
    }
}

#[test]
fn utf8_bom_optional_declaration_empty_and_explicit_empty_models_are_supported() {
    let empty = synthetic("");
    let with_header = format!("\u{feff}<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{empty}");
    let a = decode(empty.as_bytes()).unwrap();
    let b = decode(with_header.as_bytes()).unwrap();
    let c = decode(synthetic("<Attributes inPitch=\"0\"/>").as_bytes()).unwrap();
    assert_eq!(a.digest(), b.digest());
    assert_eq!(a.digest(), c.digest());
    assert!(a.outputs.iter().all(Vec::is_empty));
}

#[test]
fn maximal_fixed_midi_model_fits_byte_depth_and_output_bounds() {
    let outputs: Vec<_> = (0..128).collect();
    let xml = synthetic(
        &(0..128)
            .map(|pitch| input(pitch, &outputs))
            .collect::<String>(),
    );
    assert!(xml.len() < MAX_CAPTURE_BYTES);
    let model = decode(xml.as_bytes()).unwrap();
    assert_eq!(model.outputs.iter().map(Vec::len).sum::<usize>(), 16_384);
    assert!(model.outputs.iter().all(|value| *value == outputs));
    let value = model.page(112, 16, xml.as_bytes(), &instance());
    assert!(value.to_string().len() < MAX_RESPONSE_BYTES);
}

#[test]
fn unsupported_xml_and_model_shapes_never_return_partial_chords() {
    let valid = synthetic(&input(0, &[127]));
    let cases = [
        format!("\u{feff}\u{feff}{valid}"),
        format!("<!DOCTYPE MusicEffectPreset>{valid}"),
        format!("<?inspect value?>{valid}"),
        format!("<?xml version=\"1.1\"?>{valid}"),
        format!("<?xml version=\"1.0\" encoding=\"UTF-16\"?>{valid}"),
        format!("<?xml encoding=\"UTF-8\" version=\"1.0\"?>{valid}"),
        format!("<?xml version=\"1.0\" version=\"1.0\"?>{valid}"),
        format!("<?xml version=\"1.0\" extra=\"1\"?>{valid}"),
        format!("<?xml version=\"1.0\"?><?xml version=\"1.0\"?>{valid}"),
        format!(" <?xml version=\"1.0\"?>{valid}"),
        format!("{valid}{valid}"),
        valid.replace("<Attributes inPitch", "text<Attributes inPitch"),
        valid.replace("<Attributes inPitch", "<!--comment--><Attributes inPitch"),
        valid.replace("<Attributes inPitch", "<![CDATA[ ]]><Attributes inPitch"),
        valid.replace("<Attributes inPitch", "&#32;<Attributes inPitch"),
        valid.replace("<Attributes inPitch", "&custom;<Attributes inPitch"),
        valid.replace(
            "MusicEffectPreset cid",
            "MusicEffectPreset xmlns:x=\"urn:other\" cid",
        ),
        valid.replace("cid=", "unknown=\"value\" cid="),
        valid.replace(CLASS, "00000000-0000-0000-0000-000000000001"),
        valid.replace("x:id=\"chorder\"", "x:id=\"other\""),
        valid.replace("x:id=\"model\"", "x:id=\"other\""),
        valid.replace("outPitch=\"127\"", "outPitch=\"127\" velocity=\"1\""),
        valid.replace("outPitch=\"127\"", "outPitch=\"127\" outPitch=\"127\""),
        valid.replace("outPitch=\"127\"", "outPitch=\"128\""),
        valid.replace("outPitch=\"127\"", "outPitch=\"-1\""),
        valid.replace("outPitch=\"127\"", "outPitch=\"1.0\""),
        valid.replace("outPitch=\"127\"", "outPitch=\"+1\""),
        valid.replace("outPitch=\"127\"", "outPitch=\"&#49;\""),
        valid.replace("inPitch=\"0\"", "inPitch=\"128\""),
        valid.replace("inPitch=\"0\"", "inPitch=\"0\" unknown=\"1\""),
        valid.replace("autofill=\"0\"", "autofill=\"2\""),
        valid.replace("transpose=\"2\"", "transpose=\"NaN\""),
        valid.replace("transpose=\"2\"", "transpose=\"2\" future=\"1\""),
        valid.replace(
            "<Attributes outPitch=\"127\"/>",
            "<Attributes outPitch=\"127\"><Attributes/></Attributes>",
        ),
        valid.replace(
            "<Attributes outPitch=\"127\"/>",
            "<Attributes outPitch=\"127\"/><Attributes outPitch=\"127\"/>",
        ),
        synthetic(&(input(0, &[1]) + &input(0, &[2]))),
        valid.replace("</MusicEffectPreset>", "</Other>"),
        valid.replace(
            "<Attributes x:id=\"ComponentData\">",
            "<Other x:id=\"ComponentData\">",
        ),
        valid.replace("\"chorder\"", "\"chorder&other;\""),
        synthetic("").replace("<Attributes x:id=\"model\"></Attributes>", ""),
    ];
    for (index, xml) in cases.iter().enumerate() {
        assert!(decode(xml.as_bytes()).is_err(), "unsupported case {index}");
    }
    for bytes in [vec![0xff], vec![b' '; MAX_CAPTURE_BYTES + 1], Vec::new()] {
        assert!(decode(&bytes).is_err());
    }
    for end in 0..valid.len() {
        assert!(
            decode(&valid.as_bytes()[..end]).is_err(),
            "truncated at {end}"
        );
    }
}

#[test]
fn exact_read_request_refuses_injected_data_and_unbounded_pages() {
    let valid = json!({"operation":"chorder_chords_get","offset":0,"limit":16,"expected_instance":uid(&SERVICE_CID)});
    let request: Request = serde_json::from_value(valid.clone()).unwrap();
    assert!(request.validate().is_ok());
    assert_eq!(request.source(), Source::Auto);
    for key in [
        "path",
        "xml",
        "bytes",
        "value",
        "token",
        "source",
        "import",
        "include_owner",
    ] {
        let mut value = valid.clone();
        value[key] = json!(1);
        assert!(serde_json::from_value::<Request>(value).is_err());
    }
    for key in ["offset", "limit", "expected_instance"] {
        let mut value = valid.clone();
        value.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<Request>(value).is_err());
    }
    for (offset, limit) in [(128, 1), (u32::MAX, 16), (0, 0), (0, 17)] {
        let mut value = valid.clone();
        value["offset"] = json!(offset);
        value["limit"] = json!(limit);
        assert!(
            serde_json::from_value::<Request>(value)
                .unwrap()
                .validate()
                .is_err()
        );
    }
}

#[test]
fn late_receipt_invalidation_erases_model_and_capture_claims() {
    let xml = synthetic(&input(12, &[3, 79]));
    let value = decode(xml.as_bytes())
        .unwrap()
        .page(0, 16, xml.as_bytes(), &instance());
    let mut reply = Reply::unbound(value);
    reply.invalidate("late document switch".into());
    assert_eq!(reply.value["ok"], false);
    assert_eq!(reply.value["outcome"], "not_applied");
    for key in [
        "chords",
        "model_sha256",
        "capture_sha256",
        "capture_byte_count",
        "schema",
        "total",
        "offset",
        "limit",
        "count",
        "next",
    ] {
        assert!(reply.value.get(key).is_none(), "{key}");
    }
}
