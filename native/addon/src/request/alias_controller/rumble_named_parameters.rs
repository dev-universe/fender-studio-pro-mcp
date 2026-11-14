use super::*;

pub(super) fn lookup(controller: *mut c_void, name: &CString) -> *mut c_void {
    let name = PlainCString {
        text: name.as_ptr(),
        owner: ptr::null_mut(),
    };
    let find: unsafe extern "C" fn(*mut c_void, *const PlainCString) -> *mut c_void =
        unsafe { method(controller, 5) };
    unsafe { find(controller, &name) }
}
fn valid_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name.as_bytes()[0].is_ascii_alphabetic()
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn valid_domain(names: &[BTreeSet<String>]) -> bool {
    names.len() == 26
        && names
            .iter()
            .all(|set| set.len() <= 64 && set.iter().all(|n| valid_name(n)))
        && names.iter().map(BTreeSet::len).sum::<usize>() <= MAX_TOTAL
}
pub(in crate::native_script::request) fn failure() -> Value {
    json!({"ok":false,"outcome":"not_applied","native_outcome":"not_applied","mutation_capability":false,"can_set_via_this_action":false,"selection_authority":false,"atomic_snapshot":false,"audio_verified":false,"persistence_verified":false,"error":{"code":"rumble_named_parameters_unavailable","message":"The bounded Rumble named-parameter observation was not established"}})
}
pub(in crate::native_script::request) fn observe(
    ctx: &Context<'_>,
) -> Result<(Value, Observation), String> {
    let (before, mut observation) = capture(ctx, Retrieval::Named)?;
    if !valid_domain(&before.names) {
        return Err("Rumble named query domain exceeds policy".into());
    }
    observation.names = Some(before.names.clone());
    let mut guard = || ctx.check();
    let mut rows = Vec::with_capacity(26);
    let mut counts_complete = true;
    let mut metadata_complete = true;
    let mut current_complete = true;
    let mut matched = 0usize;
    for i in 0..26 {
        let child_index = observation.children.iter().position(|c| c.slot == Some(i));
        let capability = child_index.map(|c| &observation.children[c].capability);
        let status = capability.map_or("node_absent", |c| c.status);
        let count = capability.and_then(|c| c.count);
        let Some(child_index) = child_index.filter(|_| count.is_some()) else {
            counts_complete = false;
            metadata_complete = false;
            current_complete = false;
            rows.push(json!({"slot":slot(i),"serialized_class_uid":before.classes[i],"status":status,"parameter_count":null,"expected_names":before.names[i],"parameters":null}));
            continue;
        };
        let mut records = Vec::with_capacity(before.names[i].len());
        for name in &before.names[i] {
            observation.binding.check(&mut guard)?;
            let controller = observation.children[child_index]
                .capability
                .controller
                .as_ref()
                .ok_or("Rumble controller lost")?
                .0;
            let owned_name =
                CString::new(name.as_str()).map_err(|_| "Rumble named query invalid")?;
            guard()?;
            let pointer = lookup(controller, &owned_name);
            if pointer.is_null() {
                observation.named_holes.push((child_index, owned_name));
                guard()?;
                records.push(json!({"name":name,"status":"parameter_absent","description":null}));
                metadata_complete = false;
                current_complete = false;
                continue;
            }
            let parameter = Parameter::capture(
                pointer,
                Association::Named(owned_name),
                child_index,
                &observation,
                ctx.api,
                &mut guard,
            )?;
            parameter.check(&observation, &mut guard)?;
            let description = parameter.parameter.describe();
            guard()?;
            parameter.check(&observation, &mut guard)?;
            let failed = description.is_err();
            if description.as_ref().is_ok_and(|d| d.name != *name) {
                return Err("Rumble named description name mismatch".into());
            }
            let projected = description.ok().and_then(|d| project(&d));
            if let Some(description) = projected {
                matched += 1;
                current_complete &= description["current"]["status"] == "observed";
                records.push(json!({"name":name,"status":"observed","description":description}));
            } else {
                metadata_complete = false;
                current_complete = false;
                records.push(json!({"name":name,"status":if failed {"description_unavailable"} else {"unsupported_metadata"},"description":null}));
            }
            observation.parameters.push(parameter);
        }
        rows.push(json!({"slot":slot(i),"serialized_class_uid":before.classes[i],"status":"present","parameter_count":count,"expected_names":before.names[i],"parameters":records}));
    }
    let after = state::rumble_models(ctx, &observation.state_alias)?;
    if after.classes != before.classes || after.names != before.names {
        return Err("Rumble named model or name domain changed".into());
    }
    let expected: usize = before.names.iter().map(BTreeSet::len).sum();
    current_complete &= metadata_complete;
    let value = json!({"ok":true,"outcome":"observed","operation":"rumble_named_parameters_read","schema":"rumble_named_parameters_v1","instance":ctx.instance,"source":"component_alias_fixed_controllers","scope":"fixed_immediate_serialized_sibling_names","controllers":rows,"models_sha256":before.sha256,"before_after_model_ids_equal":true,"before_after_name_sets_equal":true,"name_set_complete":true,"controller_counts_complete":counts_complete,"named_metadata_complete":metadata_complete,"access_metadata_complete":metadata_complete,"current_values_complete":current_complete,"expected_name_count":expected,"matched_name_count":matched,"value_semantics":"sequential_snapshot","mutation_capability":false,"can_set_via_this_action":false,"selection_authority":false,"atomic_snapshot":false,"audio_verified":false,"persistence_verified":false});
    if !observation.valid_receipt(&json!(ctx.instance), &value) {
        return Err("Rumble named receipt invalid".into());
    }
    observation.check(ctx)?;
    Ok((value, observation))
}
pub(in crate::native_script::request) fn valid_receipt(instance: &Value, value: &Value) -> bool {
    if !exact(
        value,
        &[
            "ok",
            "outcome",
            "operation",
            "schema",
            "instance",
            "source",
            "scope",
            "controllers",
            "models_sha256",
            "before_after_model_ids_equal",
            "before_after_name_sets_equal",
            "name_set_complete",
            "controller_counts_complete",
            "named_metadata_complete",
            "access_metadata_complete",
            "current_values_complete",
            "expected_name_count",
            "matched_name_count",
            "value_semantics",
            "mutation_capability",
            "can_set_via_this_action",
            "selection_authority",
            "atomic_snapshot",
            "audio_verified",
            "persistence_verified",
        ],
    ) || value["ok"] != true
        || value["outcome"] != "observed"
        || value["operation"] != "rumble_named_parameters_read"
        || value["schema"] != "rumble_named_parameters_v1"
        || value["instance"] != *instance
        || value["source"] != "component_alias_fixed_controllers"
        || value["scope"] != "fixed_immediate_serialized_sibling_names"
        || value["before_after_model_ids_equal"] != true
        || value["before_after_name_sets_equal"] != true
        || value["name_set_complete"] != true
        || value["value_semantics"] != "sequential_snapshot"
        || [
            "mutation_capability",
            "can_set_via_this_action",
            "selection_authority",
            "atomic_snapshot",
            "audio_verified",
            "persistence_verified",
        ]
        .iter()
        .any(|k| value[*k] != false)
    {
        return false;
    }
    let mut adapted = value.clone();
    adapted["plugin"] = json!("Rumble Native");
    adapted["native_outcome"] = json!("observed");
    if value.to_string().len() > MAX_BYTES || adapted.to_string().len() > MAX_BYTES {
        return false;
    }
    let Some(rows) = value["controllers"].as_array().filter(|r| r.len() == 26) else {
        return false;
    };
    let mut classes = Vec::with_capacity(26);
    let mut expected = 0usize;
    let mut matched = 0usize;
    let mut counts = true;
    let mut metadata = true;
    let mut current = true;
    for (i, row) in rows.iter().enumerate() {
        if !exact(
            row,
            &[
                "slot",
                "serialized_class_uid",
                "status",
                "parameter_count",
                "expected_names",
                "parameters",
            ],
        ) || row["slot"] != slot(i)
        {
            return false;
        }
        let Some(class) = row["serialized_class_uid"].as_str() else {
            return false;
        };
        classes.push(class.to_owned());
        let Some(names) = row["expected_names"].as_array().filter(|n| n.len() <= 64) else {
            return false;
        };
        let mut previous: Option<&str> = None;
        for name in names {
            let Some(name) = name.as_str().filter(|n| valid_name(n)) else {
                return false;
            };
            if previous.is_some_and(|p| p >= name) {
                return false;
            }
            previous = Some(name);
        }
        expected += names.len();
        if expected > MAX_TOTAL {
            return false;
        }
        if row["status"] != "present" {
            if !matches!(
                row["status"].as_str(),
                Some(
                    "node_absent"
                        | "controller_absent"
                        | "qi_failed"
                        | "malformed_success"
                        | "malformed_absence"
                        | "identity_mismatch"
                        | "negative_count"
                        | "count_limit"
                )
            ) || !row["parameter_count"].is_null()
                || !row["parameters"].is_null()
            {
                return false;
            }
            counts = false;
            metadata = false;
            current = false;
            continue;
        }
        if !bounded_integer(&row["parameter_count"], 0, 128) {
            return false;
        }
        let Some(parameters) = row["parameters"]
            .as_array()
            .filter(|p| p.len() == names.len())
        else {
            return false;
        };
        for (name, p) in names.iter().zip(parameters) {
            if !exact(p, &["name", "status", "description"]) || p["name"] != *name {
                return false;
            }
            if p["status"] == "observed" {
                if !valid_description(&p["description"]) || p["description"]["name"] != *name {
                    return false;
                }
                matched += 1;
                current &= p["description"]["current"]["status"] == "observed";
            } else {
                if !matches!(
                    p["status"].as_str(),
                    Some("parameter_absent" | "description_unavailable" | "unsupported_metadata")
                ) || !p["description"].is_null()
                {
                    return false;
                }
                metadata = false;
                current = false;
            }
        }
    }
    let Some(hash) = state::rumble_model_digest(&classes) else {
        return false;
    };
    value["models_sha256"] == hash
        && value["controller_counts_complete"] == counts
        && value["named_metadata_complete"] == metadata
        && value["access_metadata_complete"] == metadata
        && value["current_values_complete"] == (current && metadata)
        && value["expected_name_count"].as_u64() == Some(expected as u64)
        && value["matched_name_count"].as_u64() == Some(matched as u64)
}
