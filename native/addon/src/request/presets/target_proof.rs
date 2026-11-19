use super::*;

struct Target {
    identity: Owned,
    view: Owned,
}
impl Target {
    fn capture(pointer: *mut c_void) -> Result<Self, String> {
        let view = hold(pointer).ok_or("preset target is unavailable")?;
        let identity = Self::identity(view.0)?;
        Ok(Self { identity, view })
    }
    fn identity(view: *mut c_void) -> Result<Owned, String> {
        optional_interface(view, &UNKNOWN)?
            .ok_or("preset target canonical identity unavailable".into())
    }
    fn check(&self) -> Result<(), String> {
        if Self::identity(self.view.0)?.0 != self.identity.0 {
            return Err("preset target canonical identity changed".into());
        }
        Ok(())
    }
}

struct ComponentAlias {
    component: Option<Target>,
    alias: Owned,
}
impl ComponentAlias {
    fn capture(device: *mut c_void) -> Result<Option<Self>, String> {
        let Some(alias) = optional_interface(device, &ALIAS)? else {
            return Ok(None);
        };
        let component = Self::get(alias.0);
        let component = (!component.is_null())
            .then(|| Target::capture(component))
            .transpose()?;
        Ok(Some(Self { alias, component }))
    }
    fn get(alias: *mut c_void) -> *mut c_void {
        let get: Borrowed = unsafe { method(alias, 3) };
        unsafe { get(alias) }
    }
    fn check_attachment(&self) -> Result<(), String> {
        let expected = self
            .component
            .as_ref()
            .map_or(ptr::null_mut(), |component| component.view.0);
        if Self::get(self.alias.0) != expected {
            return Err("preset target component alias changed".into());
        }
        Ok(())
    }
}

pub(in super::super) struct TargetProof {
    alias: Option<ComponentAlias>,
    selected: Target,
    mediator: Owned,
    route: Route,
    storable: bool,
}
impl TargetProof {
    pub(in super::super) fn capture(ctx: &Context<'_>) -> Result<Self, String> {
        node_inspect::validate_cv_target(&ctx.instance)?;
        let route = Route::capture(ctx)?;
        if route.slot_identity.2 != "AudioSynthSlot" {
            return Err("CV preset target requires the exact AudioSynthSlot".into());
        }
        Self::observe(route, &mut || ctx.check())
    }

    pub(super) fn observe<G: FnMut() -> Result<(), String>>(
        route: Route,
        guard: &mut G,
    ) -> Result<Self, String> {
        guard()?;
        let mediator = optional_interface(route.slot.0, &MEDIATOR)?
            .ok_or("CV slot IPresetMediator unavailable")?;
        guard()?;
        let selected = Target::capture(Self::get(mediator.0))?;
        guard()?;
        let alias = ComponentAlias::capture(route._target.0)?;
        let mut proof = Self {
            alias,
            selected,
            mediator,
            route,
            storable: false,
        };
        proof.check(guard)?;
        let storable = optional_interface(proof.selected.view.0, &state::STORABLE)?;
        guard()?;
        let valid = if let Some(storable) = storable.as_ref() {
            Target::identity(storable.0)?.0 == proof.selected.identity.0
        } else {
            true
        };
        proof.storable = storable.is_some();
        drop(storable);
        proof.check(guard)?;
        if !valid {
            return Err("preset target IStorable has a different canonical identity".into());
        }
        Ok(proof)
    }

    fn get(mediator: *mut c_void) -> *mut c_void {
        let get: Borrowed = unsafe { method(mediator, 4) };
        unsafe { get(mediator) }
    }

    pub(in super::super) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<(), String> {
        guard()?;
        let mediator = optional_interface(self.route.slot.0, &MEDIATOR)?;
        if mediator.as_ref().map(|interface| interface.0) != Some(self.mediator.0) {
            return Err("preset slot mediator identity changed".into());
        }
        drop(mediator);
        let alias = optional_interface(self.route._target.0, &ALIAS)?;
        if alias.as_ref().map(|interface| interface.0)
            != self.alias.as_ref().map(|alias| alias.alias.0)
        {
            return Err("preset target component alias changed".into());
        }
        drop(alias);
        self.selected.check()?;
        if let Some(component) = self
            .alias
            .as_ref()
            .and_then(|alias| alias.component.as_ref())
        {
            component.check()?;
        }
        self.route.attachment()?;
        guard()?;
        if Self::get(self.mediator.0) != self.selected.view.0 {
            return Err("preset mediator target changed".into());
        }
        if let Some(alias) = &self.alias {
            alias.check_attachment()?;
        }
        self.route.borrowed_attachment()?;
        guard()
    }

    pub(in super::super) fn value(&self) -> Value {
        let selected = self.selected.identity.0;
        let relation = |identity| {
            if selected == identity {
                "same"
            } else {
                "distinct"
            }
        };
        let alias = self
            .alias
            .as_ref()
            .and_then(|alias| alias.component.as_ref());
        json!({
            "scope":"cv_preset_target",
            "coverage":"canonical_identity_and_interface_presence",
            "mediator_target_vs_device":relation(self.route._target.0),
            "mediator_target_vs_component_alias":alias.map_or("unavailable", |alias| relation(alias.identity.0)),
            "target_istorable":if self.storable {"present"} else {"absent"},
            "primary_restore_target_verified":false,
            "atomic_snapshot":false,
            "mutation_capability":false,
        })
    }
}
