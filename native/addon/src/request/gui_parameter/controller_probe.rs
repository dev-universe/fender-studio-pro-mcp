use super::*;
use fender_native_node_api::Inspection;
use std::ffi::CStr;

const CONTROLLER_PROVIDER: Uid = Uid {
    a: 0x16ad4d2a,
    b: 0x9e14,
    c: 0x42e1,
    d: [0x9d, 0xa5, 0x72, 0xde, 0x99, 0x2b, 0xf5, 0x16],
};

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum QueryFact {
    NotAttempted,
    Unsupported,
    Present,
    Failed { query_status: i32 },
}
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum Anchor {
    NotAttempted,
    ControllerAbsent,
    ParameterAbsent,
    IdentityMismatch,
    SameParameter,
}
#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum Reason {
    ObserverAbsent,
    InterfaceQueryFailed,
    NoAnchoredController,
    AmbiguousControllerRoutes,
    ParameterMissing,
}
#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum Selected {
    Direct,
    Provider,
    Both,
}
struct InterfacePin {
    canonical: Owned,
    handle: Owned,
}
impl InterfacePin {
    fn capture<G: FnMut() -> Result<(), String>>(
        handle: Owned,
        guard: &mut G,
    ) -> Result<Self, String> {
        let canonical = required(handle.0, &UNKNOWN, guard);
        match canonical {
            Ok(canonical) => Ok(Self { canonical, handle }),
            Err(reason) => {
                release(handle, guard)?;
                Err(reason)
            }
        }
    }
    fn check<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
        witnesses: &mut Vec<Owned>,
    ) -> Result<(), String> {
        let fresh = required(self.handle.0, &UNKNOWN, guard)?;
        let same = fresh.0 == self.canonical.0;
        witnesses.push(fresh);
        if same {
            Ok(())
        } else {
            Err("GUI observer interface identity changed".into())
        }
    }
}
struct NamedPin {
    name: &'static CStr,
    parameter: Option<ParameterPin>,
}
impl NamedPin {
    fn capture<G: FnMut() -> Result<(), String>>(
        controller: *mut c_void,
        name: &'static CStr,
        api: &Api,
        guard: &mut G,
    ) -> Result<Self, String> {
        let parameter = find(controller, name, api, guard)?;
        let parameter = parameter
            .map(|parameter| ParameterPin::capture(parameter, guard))
            .transpose();
        if let Err(reason) = guard() {
            release(parameter, guard)?;
            return Err(reason);
        }
        Ok(Self {
            name,
            parameter: parameter?,
        })
    }
    fn check<G: FnMut() -> Result<(), String>>(
        &self,
        controller: *mut c_void,
        api: &Api,
        guard: &mut G,
        witnesses: &mut Vec<Owned>,
    ) -> Result<(), String> {
        let fresh = find(controller, self.name, api, guard)?;
        let result = (|| {
            match (&self.parameter, &fresh) {
                (Some(parameter), Some(fresh)) => {
                    parameter.check(guard, witnesses)?;
                    let canonical = required(fresh.as_borrowed_raw().as_ptr(), &UNKNOWN, guard)?;
                    let same = canonical.0 == parameter.canonical.0;
                    witnesses.push(canonical);
                    if !same {
                        return Err("GUI observer named parameter identity changed".into());
                    }
                }
                (None, None) => {}
                _ => return Err("GUI observer named parameter presence changed".into()),
            }
            Ok(())
        })();
        release(fresh, guard)?;
        result
    }
    fn check_borrowed(&self, controller: *mut c_void) -> Result<(), String> {
        let expected = self
            .parameter
            .as_ref()
            .map_or(ptr::null_mut(), |p| p.parameter.as_borrowed_raw().as_ptr());
        if lookup(controller, self.name) != expected {
            return Err("GUI observer named parameter association changed".into());
        }
        if let Some(parameter) = self.parameter.as_ref() {
            parameter.check_borrowed()?;
        }
        Ok(())
    }
    fn describe<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<Option<parameters::Description>, String> {
        let Some(parameter) = self.parameter.as_ref() else {
            return Ok(None);
        };
        guard()?;
        let observed = parameter.parameter.describe().map_err(|e| e.to_string());
        guard()?;
        let observed = observed?;
        if observed.name.as_bytes() != self.name.to_bytes() {
            return Err("GUI observer parameter name changed".into());
        }
        Ok(Some(observed))
    }
}
fn lookup(controller: *mut c_void, name: &'static CStr) -> *mut c_void {
    let name = PlainCString {
        text: name.as_ptr(),
        owner: ptr::null_mut(),
    };
    let find: unsafe extern "C" fn(*mut c_void, *const PlainCString) -> *mut c_void =
        unsafe { method(controller, 5) };
    unsafe { find(controller, &name) }
}
fn find<G: FnMut() -> Result<(), String>>(
    controller: *mut c_void,
    name: &'static CStr,
    api: &Api,
    guard: &mut G,
) -> Result<Option<RawParameter>, String> {
    guard()?;
    let raw = lookup(controller, name);
    let parameter = if raw.is_null() {
        Ok(None)
    } else {
        retained_parameter(raw, api).map(Some)
    };
    if let Err(reason) = guard() {
        release(parameter, guard)?;
        return Err(reason);
    }
    parameter
}
fn interface<G: FnMut() -> Result<(), String>>(
    observer: *mut c_void,
    iid: &Uid,
    guard: &mut G,
) -> Result<(QueryFact, Option<Owned>), String> {
    match query(observer, iid, guard) {
        Ok(Some(interface)) => Ok((QueryFact::Present, Some(interface))),
        Ok(None) => Ok((QueryFact::Unsupported, None)),
        Err(QueryFailure::Native(query_status)) => Ok((QueryFact::Failed { query_status }, None)),
        Err(QueryFailure::Guard(reason)) => Err(reason),
    }
}
struct Route {
    anchor: Option<NamedPin>,
    controller: Option<InterfacePin>,
    query: QueryFact,
}
impl Default for Route {
    fn default() -> Self {
        Self {
            anchor: None,
            controller: None,
            query: QueryFact::NotAttempted,
        }
    }
}
impl Route {
    fn anchor(&self, gui: &ParameterPin) -> Anchor {
        if matches!(
            self.query,
            QueryFact::NotAttempted | QueryFact::Failed { .. }
        ) {
            Anchor::NotAttempted
        } else if self.controller.is_none() {
            Anchor::ControllerAbsent
        } else {
            match self
                .anchor
                .as_ref()
                .and_then(|anchor| anchor.parameter.as_ref())
            {
                None => Anchor::ParameterAbsent,
                Some(parameter) if parameter.canonical.0 == gui.canonical.0 => {
                    Anchor::SameParameter
                }
                Some(_) => Anchor::IdentityMismatch,
            }
        }
    }
    fn capture_anchor<G: FnMut() -> Result<(), String>>(
        &mut self,
        api: &Api,
        guard: &mut G,
    ) -> Result<(), String> {
        if let Some(controller) = self.controller.as_ref() {
            self.anchor = Some(NamedPin::capture(
                controller.handle.0,
                c"autofill",
                api,
                guard,
            )?);
        }
        Ok(())
    }
    fn check<G: FnMut() -> Result<(), String>>(
        &self,
        api: &Api,
        guard: &mut G,
        witnesses: &mut Vec<Owned>,
    ) -> Result<(), String> {
        if let Some(controller) = self.controller.as_ref() {
            controller.check(guard, witnesses)?;
            self.anchor
                .as_ref()
                .ok_or("GUI observer anchor was not assessed")?
                .check(controller.handle.0, api, guard, witnesses)?;
        }
        Ok(())
    }
    fn check_borrowed(&self) -> Result<(), String> {
        if let Some(controller) = self.controller.as_ref() {
            self.anchor
                .as_ref()
                .ok_or("GUI observer anchor was not assessed")?
                .check_borrowed(controller.handle.0)?;
        }
        Ok(())
    }
}
enum State {
    Unresolved(Reason),
    Observed(Box<Observed>),
}
struct Observed {
    selected: Selected,
    observer_same: bool,
    direct_same: bool,
    mode: parameters::Description,
    pitch: parameters::Description,
}
pub(super) struct ControllerProbe {
    inspection: Option<Inspection>,
    models: Option<[NamedPin; 2]>,
    direct: Route,
    provider: Route,
    provider_interface: Option<InterfacePin>,
    anchors: [Anchor; 2],
    selected: Option<Selected>,
    state: State,
}
impl ControllerProbe {
    pub(super) fn capture<G: FnMut() -> Result<(), String>>(
        gui: &ParameterPin,
        direct_canonical: *mut c_void,
        api: &Api,
        guard: &mut G,
    ) -> Result<Self, String> {
        let mut probe = Self {
            inspection: None,
            models: None,
            direct: Route::default(),
            provider: Route::default(),
            provider_interface: None,
            anchors: [Anchor::NotAttempted; 2],
            selected: None,
            state: State::Unresolved(Reason::ObserverAbsent),
        };
        let result = probe.capture_inner(gui, direct_canonical, api, guard);
        let after = guard();
        match result.and(after) {
            Ok(()) => Ok(probe),
            Err(reason) => {
                release(probe, guard)?;
                Err(reason)
            }
        }
    }
    fn capture_inner<G: FnMut() -> Result<(), String>>(
        &mut self,
        gui: &ParameterPin,
        direct_canonical: *mut c_void,
        api: &Api,
        guard: &mut G,
    ) -> Result<(), String> {
        let Some(owner) = gui.observer.as_ref() else {
            return Ok(());
        };
        guard()?;
        self.inspection = Some(owner_inspect::reflect(
            owner,
            owner_inspect::text_api()?,
            guard,
        )?);
        let observer = unsafe { owner.as_borrowed_unknown() }
            .map_err(|e| e.to_string())?
            .as_ptr();
        let (fact, controller) = interface(observer, &CONTROLLER, guard)?;
        self.direct.query = fact;
        if let Some(controller) = controller {
            self.direct.controller = Some(InterfacePin::capture(controller, guard)?);
        }
        let (fact, provider) = interface(observer, &CONTROLLER_PROVIDER, guard)?;
        self.provider.query = fact;
        if let Some(provider) = provider {
            self.provider_interface = Some(InterfacePin::capture(provider, guard)?);
            guard()?;
            let raw = borrowed(
                self.provider_interface
                    .as_ref()
                    .expect("retained provider")
                    .handle
                    .0,
                3,
            );
            let controller = hold(raw);
            if let Err(reason) = guard() {
                release(controller, guard)?;
                return Err(reason);
            }
            if let Some(controller) = controller {
                self.provider.controller = Some(InterfacePin::capture(controller, guard)?);
            }
        }
        self.direct.capture_anchor(api, guard)?;
        self.provider.capture_anchor(api, guard)?;
        self.anchors = [self.direct.anchor(gui), self.provider.anchor(gui)];
        if matches!(self.direct.query, QueryFact::Failed { .. })
            || matches!(self.provider.query, QueryFact::Failed { .. })
        {
            self.state = State::Unresolved(Reason::InterfaceQueryFailed);
            return Ok(());
        }
        let direct = self.direct.anchor(gui) == Anchor::SameParameter;
        let provider = self.provider.anchor(gui) == Anchor::SameParameter;
        self.selected = match (direct, provider) {
            (false, false) => {
                self.state = State::Unresolved(Reason::NoAnchoredController);
                return Ok(());
            }
            (true, false) => Some(Selected::Direct),
            (false, true) => Some(Selected::Provider),
            (true, true)
                if self
                    .direct
                    .controller
                    .as_ref()
                    .expect("anchored direct")
                    .canonical
                    .0
                    == self
                        .provider
                        .controller
                        .as_ref()
                        .expect("anchored provider")
                        .canonical
                        .0 =>
            {
                Some(Selected::Both)
            }
            (true, true) => {
                self.state = State::Unresolved(Reason::AmbiguousControllerRoutes);
                return Ok(());
            }
        };
        let selected = self
            .selected_controller()
            .expect("selected anchored controller");
        let selected_handle = selected.handle.0;
        let selected_canonical = selected.canonical.0;
        let mode = NamedPin::capture(selected_handle, c"learnMode", api, guard)?;
        self.models = Some([
            mode,
            NamedPin {
                name: c"learnInputPitch",
                parameter: None,
            },
        ]);
        self.models.as_mut().expect("model pair")[1] =
            NamedPin::capture(selected_handle, c"learnInputPitch", api, guard)?;
        let models = self.models.as_ref().expect("model pair");
        let mode = models[0].describe(guard)?;
        let pitch = models[1].describe(guard)?;
        self.state = match (mode, pitch) {
            (Some(mode), Some(pitch)) => State::Observed(Box::new(Observed {
                selected: self.selected.expect("selected receiver"),
                observer_same: gui
                    .observer_canonical
                    .as_ref()
                    .is_some_and(|p| p.0 == selected_canonical),
                direct_same: direct_canonical == selected_canonical,
                mode,
                pitch,
            })),
            _ => State::Unresolved(Reason::ParameterMissing),
        };
        Ok(())
    }
    fn selected_controller(&self) -> Option<&InterfacePin> {
        match self.selected? {
            Selected::Direct | Selected::Both => self.direct.controller.as_ref(),
            Selected::Provider => self.provider.controller.as_ref(),
        }
    }
    pub(super) fn value(&self) -> Value {
        let metadata = match self.inspection.as_ref() {
            Some(inspection) => {
                let mut report =
                    node_inspect::report(inspection.report(), node_inspect::Scope::Target);
                report["coverage"] = json!("bounded_parameter_controller_metadata");
                report["scope"] = json!("parameter_controller_observation");
                report["mutation_capability"] = json!(false);
                report["limits"] = json!({"nodes":1,"depth":0,"ancestors":0,"children_per_node":1});
                json!({"status":"observed","report":report})
            }
            None => json!({"status":"absent"}),
        };
        let mut value = json!({"scope":"fixed_gui_declared_observer_controller","mutation_capability":false,
            "value_semantics":"sequential_snapshot","interfaces":{"controller":self.direct.query,"provider":self.provider.query},
            "anchors":{"controller":self.anchors[0],"provider":self.anchors[1]},"observer_metadata":metadata});
        match &self.state {
            State::Unresolved(reason) => {
                value["status"] = json!("unresolved");
                value["reason"] = json!(reason);
            }
            State::Observed(observed) => {
                value["status"] = json!("observed");
                value["binding"] = json!({"route":observed.selected,"autofill_same_gui":true,
                    "controller_same_observer":observed.observer_same,"controller_same_direct_dsp":observed.direct_same});
                value["parameters"] = json!({"learnMode":description(&observed.mode),"learnInputPitch":description(&observed.pitch)});
            }
        }
        value
    }
    pub(super) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        gui: &ParameterPin,
        api: &Api,
        guard: &mut G,
        witnesses: &mut Vec<Owned>,
    ) -> Result<(), String> {
        let Some(owner) = gui.observer.as_ref() else {
            return guard();
        };
        let observer = unsafe { owner.as_borrowed_unknown() }
            .map_err(|e| e.to_string())?
            .as_ptr();
        for (iid, expected, pinned) in [
            (
                &CONTROLLER,
                self.direct.query,
                self.direct.controller.as_ref(),
            ),
            (
                &CONTROLLER_PROVIDER,
                self.provider.query,
                self.provider_interface.as_ref(),
            ),
        ] {
            let (fact, fresh) = interface(observer, iid, guard)?;
            let same =
                fact == expected && fresh.as_ref().map(|p| p.0) == pinned.map(|p| p.handle.0);
            witnesses.extend(fresh);
            if !same {
                return Err("GUI observer interface association changed".into());
            }
        }
        if let Some(provider) = self.provider_interface.as_ref() {
            provider.check(guard, witnesses)?;
            guard()?;
            let raw = borrowed(provider.handle.0, 3);
            let fresh = hold(raw);
            witnesses.extend(fresh);
            guard()?;
            if raw
                != self
                    .provider
                    .controller
                    .as_ref()
                    .map_or(ptr::null_mut(), |p| p.handle.0)
            {
                return Err("GUI observer provider controller changed".into());
            }
        }
        self.direct.check(api, guard, witnesses)?;
        self.provider.check(api, guard, witnesses)?;
        if let Some(models) = self.models.as_ref() {
            let selected = self
                .selected_controller()
                .ok_or("GUI observer selected controller absent")?;
            for parameter in models {
                parameter.check(selected.handle.0, api, guard, witnesses)?;
            }
        }
        if let Some(inspection) = self.inspection.as_ref() {
            inspection.check(guard).map_err(|e| e.to_string())?;
        }
        guard()
    }
    pub(super) fn check_borrowed(&self, gui: &ParameterPin) -> Result<(), String> {
        gui.check_borrowed()?;
        if let Some(provider) = self.provider_interface.as_ref()
            && borrowed(provider.handle.0, 3)
                != self
                    .provider
                    .controller
                    .as_ref()
                    .map_or(ptr::null_mut(), |p| p.handle.0)
        {
            return Err("GUI observer provider association changed".into());
        }
        self.direct.check_borrowed()?;
        self.provider.check_borrowed()?;
        if let Some(models) = self.models.as_ref() {
            let selected = self
                .selected_controller()
                .ok_or("GUI observer selected controller absent")?;
            for parameter in models {
                parameter.check_borrowed(selected.handle.0)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[repr(C)]
    struct Fake {
        table: *const *const c_void,
        refs: Cell<u32>,
        status: i32,
        output: bool,
    }
    unsafe extern "C" fn retain(pointer: *mut c_void) -> u32 {
        let value = unsafe { &*pointer.cast::<Fake>() };
        value.refs.set(value.refs.get() + 1);
        value.refs.get()
    }
    unsafe extern "C" fn release(pointer: *mut c_void) -> u32 {
        let value = unsafe { &*pointer.cast::<Fake>() };
        value.refs.set(value.refs.get() - 1);
        value.refs.get()
    }
    unsafe extern "C" fn query_mock(
        pointer: *mut c_void,
        _: *const Uid,
        output: *mut *mut c_void,
    ) -> i32 {
        let value = unsafe { &*pointer.cast::<Fake>() };
        unsafe {
            *output = if !value.output {
                ptr::null_mut()
            } else if value.status == OK {
                retain(pointer);
                pointer
            } else {
                ptr::dangling_mut::<c_void>()
            };
        }
        value.status
    }
    #[test]
    fn failed_qi_nonnull_outputs_are_never_adopted_or_released() {
        let table = [
            query_mock as *const c_void,
            retain as *const c_void,
            release as *const c_void,
        ];
        for status in [FAILED, NO_INTERFACE] {
            let mut fake = Fake {
                table: table.as_ptr(),
                refs: Cell::new(1),
                status,
                output: true,
            };
            let result = query(ptr::from_mut(&mut fake).cast(), &CONTROLLER, &mut || Ok(()));
            assert!(matches!(result, Err(QueryFailure::Native(actual)) if actual == status));
            assert_eq!(fake.refs.get(), 1);
        }
    }
    #[test]
    fn query_success_owners_are_released_even_when_the_after_guard_fails() {
        let table = [
            query_mock as *const c_void,
            retain as *const c_void,
            release as *const c_void,
        ];
        let mut fake = Fake {
            table: table.as_ptr(),
            refs: Cell::new(1),
            status: OK,
            output: true,
        };
        let mut count = 0;
        let result = query(ptr::from_mut(&mut fake).cast(), &CONTROLLER, &mut || {
            count += 1;
            if count == 1 {
                Ok(())
            } else {
                Err("context changed".into())
            }
        });
        assert!(matches!(result, Err(QueryFailure::Guard(_))));
        assert_eq!(fake.refs.get(), 1);
    }
    #[test]
    fn null_query_outputs_preserve_unsupported_and_malformed_success() {
        let table = [
            query_mock as *const c_void,
            retain as *const c_void,
            release as *const c_void,
        ];
        for status in [OK, NO_INTERFACE] {
            let mut fake = Fake {
                table: table.as_ptr(),
                refs: Cell::new(1),
                status,
                output: false,
            };
            let result = query(ptr::from_mut(&mut fake).cast(), &CONTROLLER, &mut || Ok(()));
            if status == OK {
                assert!(matches!(result, Err(QueryFailure::Native(OK))));
            } else {
                assert!(matches!(result, Ok(None)));
            }
            assert_eq!(fake.refs.get(), 1);
        }
    }
}
