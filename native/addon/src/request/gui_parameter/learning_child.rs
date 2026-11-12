use super::*;
use fender_native_node_api::TextApi;

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum Reason {
    RootNotDirect,
    RootNodeUnsupported,
    InterfaceQueryFailed,
    RootShapeChanged,
    ChildAbsent,
    ChildIdMismatch,
    ChildDetached,
    ControllerUnsupported,
    ControllerIdentityMismatch,
    ParameterMissing,
}
enum State {
    Unresolved(Reason),
    Observed(Box<[parameters::Description; 2]>),
}
pub(super) struct FixedLearningChild {
    publication_witnesses: RefCell<Vec<Owned>>,
    parameters: Option<[NamedPin; 2]>,
    controller: Option<InterfacePin>,
    parent: Option<InterfacePin>,
    child: Option<InterfacePin>,
    root: Option<InterfacePin>,
    root_controller: Option<InterfacePin>,
    root_query: QueryFact,
    controller_query: QueryFact,
    root_count: Option<i32>,
    child_id: Option<String>,
    parent_assessed: bool,
    text: Option<TextApi>,
    state: State,
}
impl FixedLearningChild {
    pub(super) fn not_direct() -> Self {
        Self {
            parameters: None,
            publication_witnesses: RefCell::new(Vec::new()),
            controller: None,
            parent: None,
            child: None,
            root: None,
            root_controller: None,
            root_query: QueryFact::NotAttempted,
            controller_query: QueryFact::NotAttempted,
            root_count: None,
            child_id: None,
            parent_assessed: false,
            text: None,
            state: State::Unresolved(Reason::RootNotDirect),
        }
    }
    pub(super) fn capture<G: FnMut() -> Result<(), String>>(
        controller: &InterfacePin,
        text: TextApi,
        api: &Api,
        guard: &mut G,
    ) -> Result<Self, String> {
        let mut probe = Self::not_direct();
        probe.text = Some(text);
        let result = probe.capture_inner(controller, api, guard);
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
        expected: &InterfacePin,
        api: &Api,
        guard: &mut G,
    ) -> Result<(), String> {
        guard()?;
        let controller = hold(expected.handle.0).ok_or("GUI root controller is unavailable")?;
        self.root_controller = Some(InterfacePin::capture(controller, guard)?);
        let controller = self.root_controller.as_ref().expect("retained root");
        if controller.canonical.0 != expected.canonical.0 {
            return Err("GUI learning child root controller identity changed".into());
        }
        let (fact, root) = interface(controller.handle.0, &NODE, guard)?;
        self.root_query = fact;
        let Some(root) = root else {
            self.state = State::Unresolved(match fact {
                QueryFact::Unsupported => Reason::RootNodeUnsupported,
                _ => Reason::InterfaceQueryFailed,
            });
            return Ok(());
        };
        self.root = Some(InterfacePin::capture(root, guard)?);
        let root = self.root.as_ref().expect("retained node");
        if root.canonical.0 != controller.canonical.0 {
            return Err("GUI learning child root node identity differs".into());
        }
        let count = node_count(root.handle.0, guard)?;
        self.root_count = Some(count);
        if count != 128 {
            self.state = State::Unresolved(Reason::RootShapeChanged);
            return Ok(());
        }
        let child = find_child(root.handle.0, self.text.expect("fixed text API"), guard)?;
        let Some(child) = child else {
            self.state = State::Unresolved(Reason::ChildAbsent);
            return Ok(());
        };
        self.child = Some(InterfacePin::capture(child, guard)?);
        let child = self.child.as_ref().expect("retained child");
        self.child_id = Some(node_id(child.handle.0, guard)?);
        if self.child_id.as_deref() != Some("pitch61") {
            self.state = State::Unresolved(Reason::ChildIdMismatch);
            return Ok(());
        }
        self.parent_assessed = true;
        let parent = node_parent(child.handle.0, guard)?;
        self.parent = parent
            .map(|parent| InterfacePin::capture(parent, guard))
            .transpose()?;
        if self.parent.as_ref().map(|parent| parent.canonical.0) != Some(root.canonical.0) {
            self.state = State::Unresolved(Reason::ChildDetached);
            return Ok(());
        }
        let (fact, controller) = interface(child.handle.0, &CONTROLLER, guard)?;
        self.controller_query = fact;
        let Some(controller) = controller else {
            self.state = State::Unresolved(match fact {
                QueryFact::Unsupported => Reason::ControllerUnsupported,
                _ => Reason::InterfaceQueryFailed,
            });
            return Ok(());
        };
        self.controller = Some(InterfacePin::capture(controller, guard)?);
        let controller = self.controller.as_ref().expect("retained child controller");
        if controller.canonical.0 != child.canonical.0 {
            self.state = State::Unresolved(Reason::ControllerIdentityMismatch);
            return Ok(());
        }
        self.parameters = Some([
            NamedPin::capture(controller.handle.0, c"learnChord", api, guard)?,
            NamedPin {
                name: c"chordAssigned",
                parameter: None,
            },
        ]);
        self.parameters.as_mut().expect("local pair")[1] =
            NamedPin::capture(controller.handle.0, c"chordAssigned", api, guard)?;
        let parameters = self.parameters.as_ref().expect("local pair");
        let first = parameters[0].describe(guard)?;
        let second = parameters[1].describe(guard)?;
        self.state = match (first, second) {
            (Some(first), Some(second)) => State::Observed(Box::new([first, second])),
            _ => State::Unresolved(Reason::ParameterMissing),
        };
        Ok(())
    }
    pub(super) fn value(&self) -> Value {
        let mut value = json!({"scope":"fixed_chorder_pitch61_learning_parameters","child_id":"pitch61",
            "mutation_capability":false,"value_semantics":"sequential_snapshot","output_pitch_mapping":"not_established",
            "interfaces":{"root_node":self.root_query,"controller":self.controller_query},"root_child_count":self.root_count});
        match &self.state {
            State::Unresolved(reason) => {
                value["status"] = json!("unresolved");
                value["reason"] = json!(reason);
            }
            State::Observed(parameters) => {
                value["status"] = json!("observed");
                value["binding"] = json!({"root_same_observer":true,"parent_same_root":true,"controller_same_child":true});
                value["parameters"] = json!({"learnChord":description(&parameters[0]),"chordAssigned":description(&parameters[1])});
            }
        }
        value
    }
    pub(super) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        api: &Api,
        guard: &mut G,
        witnesses: &mut Vec<Owned>,
    ) -> Result<(), String> {
        let Some(controller) = &self.root_controller else {
            return guard();
        };
        controller.check(guard, witnesses)?;
        check_interface(
            controller.handle.0,
            &NODE,
            self.root_query,
            self.root.as_ref(),
            guard,
            witnesses,
        )?;
        let Some(root) = &self.root else {
            return guard();
        };
        root.check(guard, witnesses)?;
        if Some(node_count(root.handle.0, guard)?) != self.root_count {
            return Err("GUI learning root child count changed".into());
        }
        if self.root_count != Some(128) {
            return guard();
        }
        let fresh = find_child(root.handle.0, self.text.expect("fixed text API"), guard)?;
        let same = fresh.as_ref().map(|p| p.0) == self.child.as_ref().map(|p| p.handle.0);
        witnesses.extend(fresh);
        if !same {
            return Err("GUI fixed learning child association changed".into());
        }
        let Some(child) = &self.child else {
            return guard();
        };
        child.check(guard, witnesses)?;
        if Some(node_id(child.handle.0, guard)?) != self.child_id {
            return Err("GUI fixed learning child identifier changed".into());
        }
        if self.parent_assessed {
            let fresh = node_parent(child.handle.0, guard)?;
            let same = fresh.as_ref().map(|p| p.0) == self.parent.as_ref().map(|p| p.handle.0);
            witnesses.extend(fresh);
            if !same {
                return Err("GUI fixed learning child parent changed".into());
            }
            if let Some(parent) = &self.parent {
                parent.check(guard, witnesses)?;
            }
        }
        if self.controller_query != QueryFact::NotAttempted {
            check_interface(
                child.handle.0,
                &CONTROLLER,
                self.controller_query,
                self.controller.as_ref(),
                guard,
                witnesses,
            )?;
        }
        if let Some(controller) = &self.controller {
            controller.check(guard, witnesses)?;
            if let Some(parameters) = &self.parameters {
                for parameter in parameters {
                    parameter.check(controller.handle.0, api, guard, witnesses)?;
                }
            }
        }
        guard()
    }
    pub(super) fn check_borrowed(&self) -> Result<(), String> {
        let Some(root) = &self.root else {
            return Ok(());
        };
        if self.root_count != Some(unsafe { count_raw(root.handle.0) }) {
            return Err("GUI learning root child count changed after cleanup".into());
        }
        if self.root_count != Some(128) {
            return Ok(());
        }
        if find_child_raw(root.handle.0, self.text.expect("fixed text API"))?
            != self.child.as_ref().map_or(ptr::null_mut(), |p| p.handle.0)
        {
            return Err("GUI fixed learning child changed after cleanup".into());
        }
        let Some(child) = &self.child else {
            return Ok(());
        };
        if Some(node_id(child.handle.0, &mut || Ok(()))?) != self.child_id {
            return Err("GUI fixed learning child identifier changed after cleanup".into());
        }
        if self.parent_assessed
            && borrowed(child.handle.0, 6)
                != self.parent.as_ref().map_or(ptr::null_mut(), |p| p.handle.0)
        {
            return Err("GUI fixed learning child parent changed after cleanup".into());
        }
        if let (Some(controller), Some(parameters)) = (&self.controller, &self.parameters) {
            for parameter in parameters {
                parameter.check_borrowed(controller.handle.0)?;
            }
        }
        Ok(())
    }
    pub(super) fn retire_publication_witnesses<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<(), String> {
        let previous = std::mem::take(&mut *self.publication_witnesses.borrow_mut());
        release(previous, guard)
    }
    pub(super) fn retain_publication_interfaces(&self) -> Result<(), String> {
        // ComparisonRead performs two terminal borrowed passes. Keep their QI
        // owners through Reply -> Receipt; releasing them here could reenter the
        // host after the association checks. A new normal check retires them
        // before running any checks, so the fixed lease cannot grow unbounded.
        const MAX_PUBLICATION_WITNESSES: usize = 14;
        let Some(controller) = &self.root_controller else {
            return Ok(());
        };
        let needed = 1
            + usize::from(self.root_query != QueryFact::NotAttempted)
            + usize::from(self.root.is_some())
            + usize::from(self.child.is_some())
            + usize::from(self.parent.is_some())
            + usize::from(self.controller_query != QueryFact::NotAttempted)
            + usize::from(self.controller.is_some());
        if self.publication_witnesses.borrow().len() + needed > MAX_PUBLICATION_WITNESSES {
            return Err("GUI learning child publication witness limit exceeded".into());
        }
        let mut witnesses = Vec::with_capacity(needed);
        let mut guard = || Ok(());
        let result = (|| {
            controller.check(&mut guard, &mut witnesses)?;
            check_interface(
                controller.handle.0,
                &NODE,
                self.root_query,
                self.root.as_ref(),
                &mut guard,
                &mut witnesses,
            )?;
            if let Some(root) = &self.root {
                root.check(&mut guard, &mut witnesses)?;
            }
            if let Some(child) = &self.child {
                child.check(&mut guard, &mut witnesses)?;
                if self.controller_query != QueryFact::NotAttempted {
                    check_interface(
                        child.handle.0,
                        &CONTROLLER,
                        self.controller_query,
                        self.controller.as_ref(),
                        &mut guard,
                        &mut witnesses,
                    )?;
                }
            }
            if let Some(parent) = &self.parent {
                parent.check(&mut guard, &mut witnesses)?;
            }
            if let Some(controller) = &self.controller {
                controller.check(&mut guard, &mut witnesses)?;
            }
            Ok(())
        })();
        self.publication_witnesses.borrow_mut().extend(witnesses);
        result
    }
}
fn check_interface<G: FnMut() -> Result<(), String>>(
    object: *mut c_void,
    iid: &Uid,
    expected: QueryFact,
    pinned: Option<&InterfacePin>,
    guard: &mut G,
    witnesses: &mut Vec<Owned>,
) -> Result<(), String> {
    let (fact, fresh) = interface(object, iid, guard)?;
    let same = fact == expected && fresh.as_ref().map(|p| p.0) == pinned.map(|p| p.handle.0);
    witnesses.extend(fresh);
    if same {
        Ok(())
    } else {
        Err("GUI learning child interface association changed".into())
    }
}
unsafe fn count_raw(node: *mut c_void) -> i32 {
    let count: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { method(node, 8) };
    unsafe { count(node) }
}
fn node_count<G: FnMut() -> Result<(), String>>(
    node: *mut c_void,
    guard: &mut G,
) -> Result<i32, String> {
    guard()?;
    let value = unsafe { count_raw(node) };
    guard()?;
    if !(0..=1_000_000).contains(&value) {
        return Err("GUI learning root child count is invalid".into());
    }
    Ok(value)
}
fn node_id<G: FnMut() -> Result<(), String>>(
    node: *mut c_void,
    guard: &mut G,
) -> Result<String, String> {
    guard()?;
    let get: unsafe extern "C" fn(*mut c_void) -> *const PlainString = unsafe { method(node, 3) };
    let id = unsafe { get(node) };
    guard()?;
    let id = unsafe { id.as_ref() }.ok_or("GUI learning child identifier is unavailable")?;
    let value = copy_string(id.object, 64);
    guard()?;
    value
}
fn find_child_raw(node: *mut c_void, text: TextApi) -> Result<*mut c_void, String> {
    let name = unsafe { (text.get_constant_string)(c"pitch61".as_ptr()) };
    if name.is_null() {
        return Err("fixed learning child string is unavailable".into());
    }
    let find: unsafe extern "C" fn(*mut c_void, *const c_void) -> *mut c_void =
        unsafe { method(node, 10) };
    Ok(unsafe { find(node, name) })
}
fn find_child<G: FnMut() -> Result<(), String>>(
    node: *mut c_void,
    text: TextApi,
    guard: &mut G,
) -> Result<Option<Owned>, String> {
    guard()?;
    let name = unsafe { (text.get_constant_string)(c"pitch61".as_ptr()) };
    guard()?;
    if name.is_null() {
        return Err("fixed learning child string is unavailable".into());
    }
    let find: unsafe extern "C" fn(*mut c_void, *const c_void) -> *mut c_void =
        unsafe { method(node, 10) };
    let result = hold(unsafe { find(node, name) });
    if let Err(reason) = guard() {
        release(result, guard)?;
        return Err(reason);
    }
    Ok(result)
}
fn node_parent<G: FnMut() -> Result<(), String>>(
    node: *mut c_void,
    guard: &mut G,
) -> Result<Option<Owned>, String> {
    guard()?;
    let result = hold(borrowed(node, 6));
    if let Err(reason) = guard() {
        release(result, guard)?;
        return Err(reason);
    }
    Ok(result)
}

#[cfg(test)]
#[path = "learning_child_tests.rs"]
mod tests;
