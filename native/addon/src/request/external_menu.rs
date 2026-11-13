use super::*;
#[path = "external_menu_identity.rs"]
mod identity;
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Port {
    Send,
    Return,
}
impl Port {
    fn native(self) -> parameters::ExternalPort {
        match self {
            Self::Send => parameters::ExternalPort::Send,
            Self::Return => parameters::ExternalPort::Return,
        }
    }
    fn text(self) -> &'static str {
        match self {
            Self::Send => "send",
            Self::Return => "return",
        }
    }
}
const ALLOCATOR: Uid = Uid {
    a: 0xc489642f,
    b: 0xed26,
    c: 0x490f,
    d: [0xb6, 0x0a, 0x67, 0x88, 0x64, 0x58, 0xb0, 0xc1],
};
pub(super) struct Receipt {
    binding: parameters::MenuReadLease,
    selected: *mut c_void,
    port: Port,
    controller: Controller,
    identity: Option<identity::Identity>,
}
impl Receipt {
    pub(super) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        let mut g = || {
            if let Some(identity) = &self.identity {
                identity.check(ctx)?;
            }
            check_selected(ctx, &self.controller, self.port, self.selected)?;
            if let Some(identity) = &self.identity {
                identity.check(ctx)?;
                self.controller.check()?;
                ctx.check()?;
            }
            Ok(())
        };
        self.binding.check(&mut g).map_err(|e| e.to_string())
    }
}
fn check_selected(
    ctx: &Context<'_>,
    controller: &Controller,
    port: Port,
    pointer: *mut c_void,
) -> Result<(), String> {
    ctx.check()?;
    controller.check()?;
    let observed = controller.find(port.native().name(), ctx.api)?;
    let same = observed.as_borrowed_raw().as_ptr() == pointer;
    drop(observed);
    controller.check()?;
    ctx.check()?;
    if same {
        Ok(())
    } else {
        Err("External menu parameter was replaced".into())
    }
}
pub(super) fn observe(ctx: &Context<'_>, port: Port, provenance: bool) -> (Value, Option<Receipt>) {
    let mut phase = parameters::MenuPhase::Prepared;
    let mut receipt = None;
    let result = (|| -> Result<Value, String> {
        if !explicit_external_controller(&ctx.instance.class_uid, Source::Controller) {
            return Err("External menu requires an exact supported class".into());
        }
        let identity = if provenance {
            if !matches!(port, Port::Send) {
                return Err("provenance requires Stereo Send".into());
            }
            Some(identity::Identity::capture(ctx)?)
        } else {
            None
        };
        let controller = external_editor_controller(ctx)?;
        ctx.check()?;
        controller.check()?;
        let parameter = controller.find(port.native().name(), ctx.api)?;
        let selected = parameter.as_borrowed_raw().as_ptr();
        check_selected(ctx, &controller, port, selected)?;
        let manager =
            hold(unsafe { (ctx.api.plugin_manager)() }).ok_or("plug-in manager unavailable")?;
        check_selected(ctx, &controller, port, selected)?;
        let allocator = query(manager.0, &ALLOCATOR).ok_or("fixed IClassAllocator unavailable")?;
        check_selected(ctx, &controller, port, selected)?;
        drop(manager);
        check_selected(ctx, &controller, port, selected)?;
        let mut guard = || {
            if let Some(identity) = &identity {
                identity.check(ctx)?;
            }
            check_selected(ctx, &controller, port, selected)?;
            if let Some(identity) = &identity {
                identity.check(ctx)?;
                controller.check()?;
                ctx.check()?;
            }
            Ok(())
        };
        if provenance {
            guard()?;
        }
        let attempt = unsafe {
            if provenance {
                parameter.external_stereo_send_menu_provenance(
                    NonNull::new(allocator.0).expect("owned allocator"),
                    &ctx.instance.class_uid,
                    &mut guard,
                )
            } else {
                parameter.external_port_menu(
                    NonNull::new(allocator.0).expect("owned allocator"),
                    port.native(),
                    &mut guard,
                )
            }
        };
        phase = attempt.phase;
        drop(allocator);
        let after_allocator = if provenance { guard() } else { Ok(()) };
        let lease = attempt.lease;
        if let Some(binding) = lease {
            receipt = Some(Receipt {
                binding,
                selected,
                port,
                controller,
                identity,
            });
        }
        after_allocator?;
        let observed = attempt.result.map_err(|e| e.to_string())?;
        receipt
            .as_ref()
            .ok_or("missing parameter binding receipt")?
            .check(ctx)?;
        let rows:Vec<_>=observed.rows.iter().map(|r|json!({"path":r.path,"name":r.name,"title":r.title,"category":r.category,"checked":r.checked,"enabled":r.enabled,"has_submenu":r.has_submenu,"handler_present":r.handler_present,"kind":if r.has_submenu{"submenu"}else{"opaque"}})).collect();
        let mut result = json!({"ok":true,"outcome":"observed","operation":"external_port_menu","source":"controller","instance":ctx.instance,"port":port.text(),"parameter":{"name":port.native().name(),"tag":port.native().tag()},"menu_class":observed.menu_class,"coverage":{"complete":true,"max_rows":256,"max_depth":4,"max_total_text_bytes":32768,"max_text_bytes_each":1024,"row_count":rows.len()},"rows":rows,"menu_phase":"closed_checked","lifecycle":{"construct_calls":1,"build_calls":1,"owned_menu_references_released":true,"root_menu_final_release_count":observed.root_menu_final_release_count,"builder_final_release_count":observed.builder_final_release_count,"post_cleanup_binding_checked":true,"nested_callbacks":"not counted; prepare/build/extension/cleanup may reenter"},"selection_authority":false,"selection_calls":0,"scope":"exact_external_parameter_menu_read","persistence_verified":false,"audio_verified":false,"value_semantics":"sequential_menu_materialization"});
        if let Some(provenance) = &observed.provenance {
            result["provenance"] = provenance_value(provenance);
        }
        Ok(result)
    })();
    match result {
        Ok(v) => (v, receipt),
        Err(reason) => {
            let mut v = error(
                "external_menu_unavailable",
                &reason,
                if phase.began() {
                    "unknown"
                } else {
                    "not_applied"
                },
            );
            v["menu_phase"] = json!(phase.as_str());
            (v, receipt)
        }
    }
}
pub(super) fn invalidate(value: &mut Value) {
    if value["operation"] == "external_port_menu" || value.get("menu_phase").is_some() {
        let phase = value.get("menu_phase").cloned();
        let outcome = value["outcome"].clone();
        let failure = value["error"].clone();
        *value = json!({"ok":false,"outcome":outcome,"error":failure});
        if let Some(p) = phase {
            value["menu_phase"] = p;
        }
    }
}

fn provenance_value(value: &parameters::MenuProvenance) -> Value {
    let leaves: Vec<_> = value.leaves.iter().map(|p| {
        let end = match p.original_end {
            parameters::ProvenanceChainEnd::SelfReference => json!({"kind":"self_reference"}),
            parameters::ProvenanceChainEnd::MissingOriginal => json!({"kind":"missing_original"}),
            parameters::ProvenanceChainEnd::Cycle {index} => json!({"kind":"cycle","index":index}),
            parameters::ProvenanceChainEnd::Limit => json!({"kind":"limit"}),
        };
        json!({"path":p.path,"handler_matches_builder":p.handler_matches_builder,"data_handler_matches_builder":p.data_handler_matches_builder,"data_builder_matches_builder":p.data_builder_matches_builder,"data_parameter_is_selected":p.data_parameter_is_selected,"original_is_selected":p.original_is_selected,"original_end":end})
    }).collect();
    json!({"leaves":leaves,"menu_data_typed_values_observed":false,"selection_authority":false,"selection_calls":0,
        "proof_scope":"held_target_sequential_read_only","read_only_intent":true,"atomic_snapshot":false,"continuous_active_mixer_membership_verified":false})
}

#[cfg(test)]
mod scope_tests {
    use super::*;
    #[test]
    fn external_menu_native_provenance_never_claims_bridge_bracket_or_continuous_membership() {
        let value = parameters::MenuProvenance {
            leaves: vec![],
            menu_data_typed_values_observed: false,
            selection_authority: false,
            selection_calls: 0,
        };
        let serialized = provenance_value(&value);
        assert_eq!(serialized.as_object().unwrap().len(), 8);
        assert_eq!(
            serialized["proof_scope"],
            "held_target_sequential_read_only"
        );
        assert_eq!(serialized["read_only_intent"], true);
        assert_eq!(serialized["atomic_snapshot"], false);
        assert_eq!(
            serialized["continuous_active_mixer_membership_verified"],
            false
        );
        assert_eq!(serialized["menu_data_typed_values_observed"], false);
        assert_eq!(serialized["selection_authority"], false);
        assert_eq!(serialized["selection_calls"], 0);
    }
}
