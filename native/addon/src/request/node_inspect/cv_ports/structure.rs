use super::*;

pub(in crate::native_script::request) fn dispatch(ctx: Box<Context<'static>>) -> Reply {
    let mut reply = Reply::unbound(json!({}));
    reply.mutation = true;
    let mut phase = fender_native_parameter_api::StructurePhase::BeforePrepare;
    let result = (|| -> Result<Value, String> {
        admit(&ctx.instance)?;
        ctx.check()?;
        let inventory = parameters::CvPortParametersInventory::capture(&ctx)?;
        inventory.check(&mut || ctx.check(), &mut || ctx.check())?;
        let parameter = inventory.fixed_output_parameter()?;
        let attempt = parameter
            .inspect_structure(ctx.started + Duration::from_secs(2), &mut || {
                inventory.check(&mut || ctx.check(), &mut || ctx.check())
            });
        phase = attempt.phase;
        let prepare_calls = attempt.prepare_calls;
        let cleanup_calls = attempt.cleanup_calls;
        let cleanup_returned = attempt.cleanup_returned;
        let cleanup_elapsed_ms = attempt.cleanup_elapsed_ms;
        reply.nodes = Some(node_inspect::Inventory::CvPortParameters(inventory));
        let observed = attempt.result.map_err(|error| error.to_string())?;
        let before = parameters::observed_parameter(0, &observed.parent_before);
        let after = parameters::observed_parameter(0, &observed.parent_after);
        let children: Vec<_> = observed
            .children
            .iter()
            .enumerate()
            .map(|(index, child)| parameters::observed_parameter(index as u32, child))
            .collect();
        let value = json!({"ok":true,"outcome":"observed","operation":"cv_port_structure_inspect","source":"dsp","instance":ctx.instance,
            "scope":"fixed_output01_direct_nested_parameters","phase":"closed_checked","prepare_calls":prepare_calls,"cleanup_calls":cleanup_calls,"cleanup_returned":cleanup_returned,"cleanup_elapsed_ms":cleanup_elapsed_ms,
            "parent_before":before,"parent_after":after,"parent_details":parameters::observed_details(&observed.parent_details,true),
            "child_count":observed.child_count,"children":children,"observation_complete":observed.observation_complete,
            "status":if observed.observation_complete {"complete"} else {"child_limit"},"retained_text_bytes":observed.retained_text_bytes,
            "limits":{"deadline_ms":2000,"parents":1,"depth":1,"children":32,"name_bytes":512,"text_utf16_units":4096,"total_child_text_bytes":32768,"encoded_response_bytes":262144},
            "assignment_values_observed":false,"routing_verified":false,"selection_authority":false,"can_set_via_this_action":false,
            "mutation_capability":false,"atomic_snapshot":false,"audio_verified":false,"persistence_verified":false});
        if let Some(nodes) = &reply.nodes {
            nodes.check(&mut || ctx.check(), &mut || ctx.check())?;
        }
        Ok(value)
    })();
    reply.value = match result {
        Ok(value) => value,
        Err(reason) => {
            let mut value = error(
                "cv_structure_unavailable",
                &reason,
                if phase.began() {
                    "unknown"
                } else {
                    "not_applied"
                },
            );
            value["phase"] = json!(phase.as_str());
            value
        }
    };
    reply.context = Some(ctx);
    reply
}

pub(in crate::native_script::request) fn invalidate(value: &mut Value) {
    if value["operation"] == "cv_port_structure_inspect" {
        *value = json!({"ok":false,"outcome":"unknown","phase":"unknown","error":value["error"].clone()});
    }
}
