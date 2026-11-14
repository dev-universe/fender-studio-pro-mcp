use super::*;
use std::cell::Cell;

const COVERAGE: &str = "six_registered_level_meter_settings";
#[derive(Clone, Copy)]
struct Setting {
    name: &'static str,
    tag: i32,
    kind: parameters::Kind,
    max: i64,
}
const SETTINGS: [Setting; 6] = [
    Setting {
        name: "mode",
        tag: 1299145829,
        kind: parameters::Kind::List,
        max: 4,
    },
    Setting {
        name: "rmsLength",
        tag: 1382904652,
        kind: parameters::Kind::List,
        max: 2,
    },
    Setting {
        name: "holdLength",
        tag: 1215261772,
        kind: parameters::Kind::List,
        max: 2,
    },
    Setting {
        name: "vuHold",
        tag: 1450526828,
        kind: parameters::Kind::Toggle,
        max: 1,
    },
    Setting {
        name: "loudnessInLUFS",
        tag: 1280658799,
        kind: parameters::Kind::Toggle,
        max: 1,
    },
    Setting {
        name: "loudnessEBU18",
        tag: 1280651576,
        kind: parameters::Kind::Toggle,
        max: 1,
    },
];
fn integer(value: Option<&parameters::NativeValue>) -> Option<i64> {
    match value {
        Some(parameters::NativeValue {
            variant_tag: 1,
            value: parameters::Value::Integer(value),
        }) => Some(*value),
        _ => None,
    }
}
impl Setting {
    fn validate(self, d: &parameters::Description) -> Result<(), String> {
        if d.name != self.name
            || d.tag != self.tag
            || d.kind != self.kind
            || integer(d.min.as_ref()) != Some(0)
            || integer(d.max.as_ref()) != Some(self.max)
            || !integer(d.raw.as_ref()).is_some_and(|v| (0..=self.max).contains(&v))
        {
            return Err("Level registered setting profile changed".into());
        }
        Ok(())
    }
    fn raw(self, value: &Value) -> Result<i64, String> {
        let value = value.as_i64().or_else(|| {
            value
                .as_f64()
                .filter(|n| n.is_finite() && n.fract() == 0.0 && *n >= 0.0 && *n <= self.max as f64)
                .map(|n| n as i64)
        });
        value
            .filter(|v| (0..=self.max).contains(v))
            .ok_or("Level settings require raw Integer values within their registered range".into())
    }
}
struct Entry {
    setting: Setting,
    binding: parameters::ParameterBindingReadLease,
    owner: parameters::ControllerOwner,
    owner_view: *mut c_void,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ModeProfile {
    pub(super) mode: i64,
    pub(super) child_count: i32,
}
impl ModeProfile {
    fn for_mode(mode: i64) -> Result<Self, String> {
        let child_count = match mode {
            0..=3 => 25,
            4 => 23,
            _ => return Err("Level mode has no observed controller count profile".into()),
        };
        Ok(Self { mode, child_count })
    }
    fn check_mode(self, description: &parameters::Description) -> Result<(), String> {
        SETTINGS[0].validate(description)?;
        if integer(description.raw.as_ref()) != Some(self.mode) {
            return Err("Level mode changed outside its setter".into());
        }
        Ok(())
    }
}
fn terminal_counts(pair: &Pair, profile: ModeProfile) -> Result<(), String> {
    for (controller, expected) in pair.capabilities.iter().zip([8, profile.child_count]) {
        let controller = controller
            .interface
            .as_ref()
            .ok_or("Level controller unavailable")?;
        let count: Count = unsafe { method(controller.0, 3) };
        if unsafe { count(controller.0) } != expected {
            return Err("Level controller count changed".into());
        }
    }
    Ok(())
}
pub(super) fn check_counts<G: FnMut() -> Result<(), String>>(
    pair: &Pair,
    profile: ModeProfile,
    guard: &mut G,
) -> Result<(), String> {
    if pair.capabilities[0].value["count"] != 8
        || !matches!(pair.capabilities[1].value["count"].as_i64(), Some(23 | 25))
    {
        return Err("Level controller counts differ from the proven profile".into());
    }
    guard()?;
    terminal_counts(pair, profile)?;
    guard()?;
    terminal_counts(pair, profile)
}
pub(super) fn dispatch_mode_change<G, S>(
    profile: &Cell<Option<ModeProfile>>,
    target: ModeProfile,
    guard: &mut G,
    setter: S,
) -> Result<(), String>
where
    G: FnMut() -> Result<(), String>,
    S: FnOnce(),
{
    guard()?;
    setter();
    profile.set(Some(target));
    guard()
}
pub(super) fn check_named(
    controller: *mut c_void,
    parameter: *mut c_void,
    owner_view: *mut c_void,
    name: &str,
) -> Result<(), String> {
    let name = CString::new(name).map_err(|_| "invalid Level setting name")?;
    let name = PlainCString {
        text: name.as_ptr(),
        owner: ptr::null_mut(),
    };
    let find: unsafe extern "C" fn(*mut c_void, *const PlainCString) -> *mut c_void =
        unsafe { method(controller, 5) };
    if unsafe { find(controller, &name) } != parameter || borrowed(parameter, 14) != owner_view {
        return Err("Level named parameter or owner attachment changed".into());
    }
    Ok(())
}
pub(in crate::native_script::request) struct Settings {
    entries: Vec<Entry>,
    inventory: Inventory,
    profile: Cell<Option<ModeProfile>>,
}
impl Settings {
    pub(in crate::native_script::request) fn registered(name: &str) -> Result<(), String> {
        SETTINGS
            .iter()
            .any(|s| s.name == name)
            .then_some(())
            .ok_or("unregistered Level setting".into())
    }
    fn pair(&self) -> Result<&Pair, String> {
        match &self.inventory {
            Inventory::Present {
                level: Some(level), ..
            } => level
                .pair
                .as_ref()
                .ok_or("unsupported Level controller shape".into()),
            _ => Err("exact Level controller is unavailable".into()),
        }
    }
    fn controller(&self) -> Result<*mut c_void, String> {
        self.pair()?.capabilities[1]
            .interface
            .as_ref()
            .map(|p| p.0)
            .ok_or("Level child controller is unavailable".into())
    }
    pub(in crate::native_script::request) fn capture(ctx: &Context<'_>) -> Result<Self, String> {
        if ctx.instance.class_uid != "DF958D90-0196-434F-ADC3-1FC2BB7F4AF6"
            || ctx.instance.native_class != "AudioEffect"
            || ctx.instance.object_id != "Level Meter"
        {
            return Err("level_meter requires the exact Level Meter AudioEffect class".into());
        }
        let text = TextApi {
            get_constant_string: unsafe {
                std::mem::transmute::<
                    ConstantString,
                    unsafe extern "C" fn(*const c_char) -> *const c_void,
                >(constant_string().ok_or("native string constants unavailable")?)
            },
        };
        let inventory = super::super::capture_bound(
            &ctx.instance,
            ctx.target.0,
            ctx.node.0,
            text,
            node_inspect::limits(node_inspect::Scope::PluginController),
            &mut || ctx.check(),
        )?;
        let mut result = Self {
            entries: Vec::new(),
            inventory,
            profile: Cell::new(None),
        };
        result.check(&mut || ctx.check())?;
        for setting in SETTINGS {
            let controller = Controller {
                handle: pin(result.controller()?, &mut || ctx.check())?,
                lease: None,
            };
            let parameter = controller.find(setting.name, ctx.api);
            result.check(&mut || ctx.check())?;
            let parameter = parameter?;
            let binding = parameter
                .pin_binding_read(&mut || result.check(&mut || ctx.check()))
                .map_err(|e| e.to_string())?;
            let owner = binding
                .parameter()
                .controller_owner(&mut || result.check(&mut || ctx.check()))
                .map_err(|e| e.to_string())?
                .ok_or("Level setting owner unavailable")?;
            let owner_view = unsafe { owner.as_borrowed_unknown() }
                .map_err(|e| e.to_string())?
                .as_ptr();
            let identity = interface(owner_view, &UNKNOWN, &mut || {
                result.check(&mut || ctx.check())
            })?;
            let matches = identity.0 == result.pair()?.child_identity.0;
            release(identity, &mut || result.check(&mut || ctx.check()))?;
            if !matches {
                return Err("Level setting owner canonical identity mismatch".into());
            }
            result.entries.push(Entry {
                setting,
                binding,
                owner,
                owner_view,
            });
            release(controller, &mut || result.check(&mut || ctx.check()))?;
            result.check(&mut || ctx.check())?;
        }
        let mode = result
            .entry("mode")?
            .binding
            .parameter()
            .describe()
            .map_err(|e| e.to_string())?;
        let profile = ModeProfile::for_mode(
            integer(mode.raw.as_ref()).ok_or("Level mode Integer unavailable")?,
        )?;
        if result.pair()?.capabilities[1].value["count"] != profile.child_count {
            return Err("Level captured mode/count profile changed".into());
        }
        result.profile.set(Some(profile));
        result.check(&mut || ctx.check())?;
        Ok(result)
    }
    pub(in crate::native_script::request) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<(), String> {
        self.inventory.check(guard)?;
        let pair = self.pair()?;
        let profile = self.profile.get().unwrap_or(ModeProfile {
            mode: -1,
            child_count: pair.capabilities[1].value["count"]
                .as_i64()
                .ok_or("Level child count unavailable")? as i32,
        });
        check_counts(pair, profile, guard)?;
        for entry in &self.entries {
            entry.binding.check(guard).map_err(|e| e.to_string())?;
            entry.owner.check(guard).map_err(|e| e.to_string())?;
            guard()?;
            let d = entry
                .binding
                .parameter()
                .describe()
                .map_err(|e| e.to_string())?;
            guard()?;
            entry.setting.validate(&d)?;
            if entry.setting.name == "mode" && self.profile.get().is_some() {
                profile.check_mode(&d)?;
            }
            self.named(entry)?;
            guard()?;
        }
        self.inventory.check(guard)?;
        for _ in 0..2 {
            guard()?;
            if let Some(profile) = self.profile.get() {
                let mode = self
                    .entry("mode")?
                    .binding
                    .parameter()
                    .describe()
                    .map_err(|e| e.to_string())?;
                profile.check_mode(&mode)?;
            }
            terminal_counts(pair, profile)?;
            for entry in &self.entries {
                self.named(entry)?;
            }
            if let Inventory::Present {
                binding,
                level: Some(level),
                ..
            } = &self.inventory
            {
                level.attachments()?;
                let alias = &binding.alias;
                if borrowed(alias.alias.0, 3) != alias.plugin.0
                    || borrowed(alias.node.0, 6) != alias.parent.0
                    || borrowed(alias.provider.0, 3) != binding.controller.0
                {
                    return Err("Level declared controller attachment changed".into());
                }
            }
        }
        Ok(())
    }
    fn set_mode<G: FnMut() -> Result<(), String>>(&self, value: i64, guard: &mut G) -> Value {
        let mut before = None;
        let mut after = None;
        let mut dispatched = false;
        let result: Result<bool, String> = (|| {
            let entry = self.entry("mode")?;
            let target = ModeProfile::for_mode(value)?;
            self.check(guard)?;
            let d = entry
                .binding
                .parameter()
                .describe()
                .map_err(|e| e.to_string())?;
            entry.setting.validate(&d)?;
            if !d.enabled || d.read_only || !d.support.raw_number {
                return Err("Level mode is not natively writable".into());
            }
            before = Some(d);
            let pointer = entry.binding.parameter().as_borrowed_raw().as_ptr();
            let enabled: unsafe extern "C" fn(*mut c_void) -> u8 = unsafe { method(pointer, 6) };
            let state: unsafe extern "C" fn(*mut c_void, i32) -> u8 = unsafe { method(pointer, 8) };
            if unsafe { enabled(pointer) } == 0 || unsafe { state(pointer, 0x1000) } != 0 {
                return Err("Level mode is not natively writable".into());
            }
            let input = VariantOut {
                kind: 1,
                payload: value as u64,
            };
            let set: unsafe extern "C" fn(*mut c_void, *const VariantOut, u8) =
                unsafe { method(pointer, 21) };
            dispatch_mode_change(&self.profile, target, &mut || self.check(guard), || {
                dispatched = true;
                unsafe { set(pointer, &input, 1) };
            })?;
            let d = entry
                .binding
                .parameter()
                .describe()
                .map_err(|e| e.to_string())?;
            entry.setting.validate(&d)?;
            let verified = integer(d.raw.as_ref()) == Some(value);
            after = Some(d);
            self.check(guard)?;
            Ok(verified)
        })();
        let mut response = match result {
            Ok(verified) => json!({"ok":true,"verified":verified,"outcome":"observed"}),
            Err(reason) => error(
                "native_parameter_error",
                &reason,
                if dispatched { "unknown" } else { "not_applied" },
            ),
        };
        if let Some(before) = before {
            response["before"] = description(&before);
        }
        if let Some(after) = after {
            response["after"] = description(&after);
        }
        response
    }
    fn named(&self, entry: &Entry) -> Result<(), String> {
        check_named(
            self.controller()?,
            entry.binding.parameter().as_borrowed_raw().as_ptr(),
            entry.owner_view,
            entry.setting.name,
        )
    }
    fn entry(&self, name: &str) -> Result<&Entry, String> {
        self.entries
            .iter()
            .find(|e| e.setting.name == name)
            .ok_or("unregistered Level setting".into())
    }
    pub(in crate::native_script::request) fn page(
        &self,
        ctx: &Context<'_>,
        offset: u32,
        limit: u32,
    ) -> Result<Value, String> {
        if offset > 6 {
            return Err("offset exceeds registered Level setting count".into());
        }
        let end = offset.saturating_add(limit).min(6);
        let mut records = Vec::new();
        for index in offset..end {
            self.check(&mut || ctx.check())?;
            let d = self.entries[index as usize]
                .binding
                .parameter()
                .describe()
                .map_err(|e| e.to_string())?;
            self.check(&mut || ctx.check())?;
            let mut value = description(&d);
            value["index"] = json!(index);
            records.push(value);
        }
        Ok(
            json!({"parameters":records,"reported_count":6,"offset":offset,"next_offset":if end<6 {Some(end)}else{None},"page_complete":true,"all_indices_read":offset==0&&end==6,"failed_indices":[],"coverage":COVERAGE,"access_metadata_complete":true}),
        )
    }
    pub(in crate::native_script::request) fn execute(
        &self,
        ctx: &Context<'_>,
        request: Request,
    ) -> Value {
        if let Request::SetParameter {
            param, mode, value, ..
        } = request
        {
            let prepared: Result<(&Entry, i64), String> = (|| {
                let entry = self.entry(&param)?;
                if !matches!(mode, Mode::Raw) {
                    return Err("Level setting writes require raw Integer values".into());
                }
                let value = entry.setting.raw(&value)?;
                self.check(&mut || ctx.check())?;
                Ok((entry, value))
            })();
            let (entry, value) = match prepared {
                Ok(v) => v,
                Err(e) => return error("invalid_level_setting", &e, "not_applied"),
            };
            if entry.setting.name == "mode" {
                return self.set_mode(value, &mut || ctx.check());
            }
            return match entry
                .binding
                .parameter()
                .set_raw_number(parameters::Number::Integer(value), &mut || {
                    self.check(&mut || ctx.check())
                }) {
                Ok(r) => {
                    json!({"ok":true,"before":description(&r.before),"after":description(&r.after),"verified":r.verified,"outcome":"observed"})
                }
                Err(e) => {
                    let mut v = error(
                        "native_parameter_error",
                        &e.error.to_string(),
                        if e.outcome == parameters::Outcome::Unknown {
                            "unknown"
                        } else {
                            "not_applied"
                        },
                    );
                    if let Some(d) = e.before {
                        v["before"] = description(&d);
                    }
                    if let Some(d) = e.after {
                        v["after"] = description(&d);
                    }
                    v
                }
            };
        }
        let result: Result<Value, String> = (|| {
            self.check(&mut || ctx.check())?;
            let value = match request {
                Request::Parameters { offset, limit, .. } => self.page(ctx, offset, limit)?,
                Request::Parameter { param, .. } => {
                    let p = self.entry(&param)?.binding.parameter();
                    let d = p.describe().map_err(|e| e.to_string())?;
                    json!({"parameter":description(&d)})
                }
                Request::Choices {
                    param,
                    offset,
                    limit,
                    ..
                } => {
                    let e = self.entry(&param)?;
                    let count = (e.setting.max + 1) as u32;
                    if offset > count {
                        return Err("offset exceeds registered Level setting choices".into());
                    }
                    let (choices, next) = if e.setting.kind == parameters::Kind::List {
                        let page = e
                            .binding
                            .parameter()
                            .list_choices(offset, limit, &mut || self.check(&mut || ctx.check()))
                            .map_err(|e| e.to_string())?;
                        if page.count != count {
                            return Err("Level setting choice count changed".into());
                        }
                        (
                            page.choices
                                .iter()
                                .map(|c| json!({"index":c.index,"text":c.label}))
                                .collect::<Vec<_>>(),
                            page.next,
                        )
                    } else {
                        let end = offset.saturating_add(limit).min(count);
                        let values = (offset..end)
                            .map(|n| parameters::Number::Integer(i64::from(n)))
                            .collect::<Vec<_>>();
                        let choices = if values.is_empty() {
                            Vec::new()
                        } else {
                            let formatted = e
                                .binding
                                .parameter()
                                .format_values(&values, &mut || self.check(&mut || ctx.check()))
                                .map_err(|e| e.to_string())?;
                            formatted
                                .values
                                .iter()
                                .zip(offset..end)
                                .map(|(v, index)| json!({"index":index,"text":v.label}))
                                .collect()
                        };
                        (choices, if end < count { Some(end) } else { None })
                    };
                    json!({"param":param,"choices":choices,"reported_count":count,"offset":offset,"next_offset":next,"page_complete":true,"all_indices_read":offset==0&&next.is_none(),"coverage":COVERAGE})
                }
                Request::FormatValues { param, values, .. } => {
                    let e = self.entry(&param)?;
                    let values = values
                        .into_iter()
                        .map(|v| e.setting.raw(&json!(v)).map(parameters::Number::Integer))
                        .collect::<Result<Vec<_>, _>>()?;
                    let formatted = e
                        .binding
                        .parameter()
                        .format_values(&values, &mut || self.check(&mut || ctx.check()))
                        .map_err(|e| e.to_string())?;
                    format::report(&param, &formatted, COVERAGE)
                }
                _ => return Err("unsupported Level settings operation".into()),
            };
            self.check(&mut || ctx.check())?;
            Ok(value)
        })();
        match result {
            Ok(mut v) => {
                v["ok"] = json!(true);
                v
            }
            Err(e) => error("native_query_failed", &e, "not_applied"),
        }
    }
}

#[cfg(test)]
#[path = "level_settings_tests.rs"]
mod tests;
