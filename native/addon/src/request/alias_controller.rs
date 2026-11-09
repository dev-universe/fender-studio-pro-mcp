//! QI views and canonical identities are guarded observations before the final
//! context guard. Borrowed attachment getters follow that guard. Retained owners
//! preserve metadata lifetime; this non-atomic report never authorizes a write.
use super::*;
#[cfg(test)]
use fender_native_node_api::Report;
use fender_native_node_api::{Inspection, Limits, NodeInspector, TextApi};
#[path = "alias_controller/catalog.rs"]
pub(super) mod catalog;

const CONTROLLER_PROVIDER: Uid = Uid {
    a: 0x16ad4d2a,
    b: 0x9e14,
    c: 0x42e1,
    d: [0x9d, 0xa5, 0x72, 0xde, 0x99, 0x2b, 0xf5, 0x16],
};

fn supported(class: &str) -> bool {
    matches!(
        class,
        "11B8AA91-9BED-4A7F-90F4-02C6A6C7AF34" | "B9D7536C-A010-457D-A182-36E5531164C9"
    )
}

pub(super) struct AliasBinding {
    provider_identity: Owned,
    provider: Owned,
    plugin_identity: Owned,
    plugin: Owned,
    alias: Owned,
    parent_identity: Owned,
    parent: Owned,
    node_identity: Owned,
    node: Owned,
}

pub(super) struct ControllerBinding {
    identity: Owned,
    controller: Owned,
    alias: AliasBinding,
}

pub(super) enum Inventory {
    Absent(AliasBinding),
    Present {
        inspection: Inspection,
        binding: ControllerBinding,
    },
}

fn release<T, G: FnMut() -> Result<(), String>>(value: T, guard: &mut G) -> Result<(), String> {
    let before = guard();
    drop(value);
    let after = guard();
    before.and(after)
}

fn pin<G: FnMut() -> Result<(), String>>(
    pointer: *mut c_void,
    guard: &mut G,
) -> Result<Owned, String> {
    let pinned = hold(pointer).ok_or("alias controller route object unavailable".to_owned());
    if let Err(reason) = guard() {
        release(pinned, guard)?;
        return Err(reason);
    }
    pinned
}

fn interface<G: FnMut() -> Result<(), String>>(
    object: *mut c_void,
    iid: &Uid,
    guard: &mut G,
) -> Result<Owned, String> {
    guard()?;
    let mut out = ptr::null_mut();
    let result = unsafe { (unknown_vtable(object).query)(object, iid, &mut out) };
    let owned = if out.is_null() {
        None
    } else {
        Some(Owned(out))
    };
    if let Err(reason) = guard() {
        release(owned, guard)?;
        return Err(reason);
    }
    if result != OK || owned.is_none() {
        release(owned, guard)?;
        return Err("alias controller route interface unavailable".into());
    }
    Ok(owned.expect("successful nonnull interface"))
}

fn borrowed(object: *mut c_void, slot: usize) -> *mut c_void {
    let get: Borrowed = unsafe { method(object, slot) };
    unsafe { get(object) }
}

impl AliasBinding {
    fn capture<G: FnMut() -> Result<(), String>>(
        target: *mut c_void,
        node: *mut c_void,
        guard: &mut G,
    ) -> Result<Self, String> {
        guard()?;
        let result = (|| {
            let node = pin(node, guard)?;
            let node_identity = interface(node.0, &UNKNOWN, guard)?;
            let parent = pin(borrowed(node.0, 6), guard)?;
            let parent_identity = interface(parent.0, &UNKNOWN, guard)?;
            let alias = interface(target, &ALIAS, guard)?;
            let plugin = pin(borrowed(alias.0, 3), guard)?;
            let plugin_identity = interface(plugin.0, &UNKNOWN, guard)?;
            let provider = interface(plugin.0, &CONTROLLER_PROVIDER, guard)?;
            let provider_identity = interface(provider.0, &UNKNOWN, guard)?;
            if provider_identity.0 != plugin_identity.0 {
                return Err(
                    "declared controller provider has a different canonical alias identity".into(),
                );
            }
            Ok(Self {
                provider_identity,
                provider,
                plugin_identity,
                plugin,
                alias,
                parent_identity,
                parent,
                node_identity,
                node,
            })
        })();
        if let Err(reason) = guard() {
            release(result, guard)?;
            return Err(reason);
        }
        result
    }

    fn check<G: FnMut() -> Result<(), String>>(
        &self,
        expected_controller: *mut c_void,
        guard: &mut G,
    ) -> Result<(), String> {
        let temporary = (|| {
            let plugin = interface(self.plugin.0, &UNKNOWN, guard)?;
            let provider = interface(self.plugin.0, &CONTROLLER_PROVIDER, guard)?;
            let provider_identity = interface(provider.0, &UNKNOWN, guard)?;
            let parent = interface(self.parent.0, &UNKNOWN, guard)?;
            let node = interface(self.node.0, &UNKNOWN, guard)?;
            let matches = plugin.0 == self.plugin_identity.0
                && provider.0 == self.provider.0
                && provider_identity.0 == self.provider_identity.0
                && parent.0 == self.parent_identity.0
                && node.0 == self.node_identity.0;
            Ok::<_, String>((matches, [plugin, provider, provider_identity, parent, node]))
        })();
        let matches = temporary.as_ref().is_ok_and(|(matches, _)| *matches);
        let reason = temporary.as_ref().err().cloned();
        release(temporary, guard)?;
        if let Some(reason) = reason {
            return Err(reason);
        }
        if !matches {
            return Err("alias controller route canonical identity changed".into());
        }
        guard()?;
        // The matching views are pinned. No temporary owner or guard follows
        // these final borrowed attachment observations.
        let plugin = borrowed(self.alias.0, 3);
        let parent = borrowed(self.node.0, 6);
        let controller = borrowed(self.provider.0, 3);
        if plugin != self.plugin.0 || parent != self.parent.0 || controller != expected_controller {
            return Err("alias controller route attachment changed".into());
        }
        Ok(())
    }
}

impl ControllerBinding {
    fn capture<G: FnMut() -> Result<(), String>>(
        alias: AliasBinding,
        guard: &mut G,
    ) -> Result<Self, String> {
        guard()?;
        let controller = pin(borrowed(alias.provider.0, 3), guard)?;
        let identity = interface(controller.0, &UNKNOWN, guard)?;
        let binding = Self {
            identity,
            controller,
            alias,
        };
        binding.check(guard)?;
        Ok(binding)
    }

    fn check<G: FnMut() -> Result<(), String>>(&self, guard: &mut G) -> Result<(), String> {
        let canonical = interface(self.controller.0, &UNKNOWN, guard);
        let matches = canonical
            .as_ref()
            .is_ok_and(|identity| identity.0 == self.identity.0);
        let reason = canonical.as_ref().err().cloned();
        release(canonical, guard)?;
        if let Some(reason) = reason {
            return Err(reason);
        }
        if !matches {
            return Err("declared controller canonical identity changed".into());
        }
        self.alias.check(self.controller.0, guard)
    }
}

impl Inventory {
    pub(super) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<(), String> {
        match self {
            Self::Absent(binding) => binding.check(ptr::null_mut(), guard),
            Self::Present {
                inspection,
                binding,
            } => {
                inspection
                    .check(&mut || binding.check(guard))
                    .map_err(|error| error.to_string())?;
                binding.check(guard)
            }
        }
    }

    #[cfg(test)]
    pub(super) fn report(&self) -> Option<&Report> {
        match self {
            Self::Absent(_) => None,
            Self::Present { inspection, .. } => Some(inspection.report()),
        }
    }

    pub(super) fn value(&self) -> Value {
        let limits = node_inspect::limits(node_inspect::Scope::PluginController);
        let mut value = match self {
            Self::Absent(_) => json!({
                "complete":false,"controller":null,"nodes":[],"edges":[],
                "diagnostics":[{"kind":"controller_absent"}],
                "limits":{"nodes":limits.nodes,"depth":limits.depth,
                    "ancestors":limits.ancestors,"children_per_node":limits.children_per_node},
            }),
            Self::Present { inspection, .. } => {
                node_inspect::report(inspection.report(), node_inspect::Scope::PluginController)
            }
        };
        value["coverage"] = json!("bounded_alias_declared_controller_metadata");
        value["scope"] = json!("alias_declared_controller_observation");
        value["mutation_capability"] = json!(false);
        value["atomic_snapshot"] = json!(false);
        value
    }
}

fn observe<G: FnMut() -> Result<(), String>>(
    alias: AliasBinding,
    text: TextApi,
    limits: Limits,
    guard: &mut G,
) -> Result<Inventory, String> {
    guard()?;
    let pointer = borrowed(alias.provider.0, 3);
    if pointer.is_null() {
        let inventory = Inventory::Absent(alias);
        inventory.check(guard)?;
        return Ok(inventory);
    }
    let controller = pin(pointer, guard)?;
    let identity = interface(controller.0, &UNKNOWN, guard)?;
    let binding = ControllerBinding {
        identity,
        controller,
        alias,
    };
    binding.check(guard)?;
    let root = pin(binding.controller.0, guard)?;
    let pointer = NonNull::new(root.0).expect("retained controller");
    std::mem::forget(root);
    let inspector = unsafe { NodeInspector::from_owned(pointer, text) };
    let inspected = inspector.inspect(limits, &mut || binding.check(guard));
    release(inspector, guard)?;
    let inspection = inspected.map_err(|error| error.to_string())?;
    let inventory = Inventory::Present {
        inspection,
        binding,
    };
    if let Err(reason) = inventory.check(guard) {
        release(inventory, guard)?;
        return Err(reason);
    }
    Ok(inventory)
}

pub(super) fn capture(
    ctx: &Context<'_>,
    text: TextApi,
    limits: Limits,
) -> Result<Inventory, String> {
    ctx.check()?;
    if !supported(&ctx.instance.class_uid) {
        return Err(
            "plugin_controller scope requires the observed Mustang Native or Rumble Native class"
                .into(),
        );
    }
    let mut guard = || ctx.check();
    let result = AliasBinding::capture(ctx.target.0, ctx.node.0, &mut guard)
        .and_then(|binding| observe(binding, text, limits, &mut guard));
    if let Err(reason) = guard() {
        release(result, &mut guard)?;
        return Err(reason);
    }
    result
}

#[cfg(test)]
#[path = "alias_controller_tests.rs"]
mod tests;
