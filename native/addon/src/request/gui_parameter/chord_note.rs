use super::*;

pub(super) struct Selection {
    mode: parameters::ScalarReadLease,
    pitch: parameters::ScalarReadLease,
}
impl Selection {
    pub(super) fn capture<G: FnMut() -> Result<(), String>>(
        mode: RawParameter,
        pitch: RawParameter,
        expected_input: u8,
        guard: &mut G,
    ) -> Result<Self, String> {
        if expected_input > 127 {
            return Err("expected input is outside MIDI0..127".into());
        }
        let mode = mode
            .pin_raw_toggle_state("learnMode", true, guard)
            .map_err(|error| error.to_string())?;
        let description = mode.description();
        let integer = |value: Option<&parameters::NativeValue>| match value {
            Some(parameters::NativeValue {
                variant_tag: 1,
                value: parameters::Value::Integer(value),
            }) => Some(*value),
            _ => None,
        };
        if description.tag != 5
            || integer(description.raw.as_ref()) != Some(1)
            || integer(description.default_value.as_ref()) != Some(0)
        {
            return Err(
                "learning membership requires exact Integer Mode1 and its declared profile".into(),
            );
        }
        let pitch = pitch
            .pin_raw_integer_state(gui_model::PITCH_PROFILE, i64::from(expected_input), guard)
            .map_err(|error| error.to_string())?;
        let selected = Self { mode, pitch };
        selected.check(guard)?;
        Ok(selected)
    }
    pub(super) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<(), String> {
        self.mode.check(guard).map_err(|error| error.to_string())?;
        self.pitch.check(guard).map_err(|error| error.to_string())?;
        guard()?;
        self.check_borrowed()
    }
    pub(super) fn check_borrowed(&self) -> Result<(), String> {
        for _ in 0..2 {
            self.mode
                .check_borrowed()
                .map_err(|error| error.to_string())?;
            self.pitch
                .check_borrowed()
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
    fn check_bindings_borrowed(&self) -> Result<(), String> {
        self.mode
            .check_binding_borrowed()
            .map_err(|e| e.to_string())?;
        self.pitch
            .check_binding_borrowed()
            .map_err(|e| e.to_string())
    }
}

pub(super) struct NotePins {
    pub(super) selected: Selection,
    pub(super) child: controller_probe::LearningNoteChild,
    pub(super) member: parameters::ScalarBindingLease,
}
impl NotePins {
    fn prepare_terminal<B: Fn() -> Result<(), String>>(&self, borrowed: &B) -> Result<(), String> {
        self.check_borrowed(borrowed)?;
        self.child.retain_publication()?;
        for _ in 0..2 {
            borrowed()?;
            self.member
                .check_binding_borrowed()
                .map_err(|e| e.to_string())?;
            self.selected.check_bindings_borrowed()?;
            self.child.check_borrowed()?;
        }
        borrowed()
    }
    pub(super) fn check<G: Fn() -> Result<(), String>, B: Fn() -> Result<(), String>>(
        &self,
        api: &Api,
        full: &G,
        borrowed: &B,
    ) -> Result<(), String> {
        full()?;
        self.selected.check(&mut || borrowed())?;
        self.member
            .check(&mut || {
                borrowed()?;
                self.selected.check_borrowed()
            })
            .map_err(|error| error.to_string())?;
        self.child.check(api, &mut || {
            borrowed()?;
            self.selected.check_borrowed()
        })?;
        self.check_borrowed(borrowed)
    }
    pub(super) fn check_borrowed<B: Fn() -> Result<(), String>>(
        &self,
        borrowed: &B,
    ) -> Result<(), String> {
        for _ in 0..2 {
            borrowed()?;
            self.child.check_borrowed()?;
            self.member
                .check_borrowed()
                .map_err(|error| error.to_string())?;
            self.selected.check_borrowed()?;
        }
        Ok(())
    }
    pub(super) fn edit<G: Fn() -> Result<(), String>, B: Fn() -> Result<(), String>>(
        &self,
        parameter: RawParameter,
        enabled: bool,
        api: &Api,
        full: &G,
        borrowed: &B,
    ) -> parameters::NumericEditAttempt {
        parameter.edit_raw_toggle_with_terminal_guard(
            "learnChord",
            enabled,
            &mut || self.check(api, full, borrowed),
            &mut || self.prepare_terminal(borrowed),
        )
    }
}

pub(in super::super) struct NoteBinding {
    base: ModelBinding,
    pins: NotePins,
}
impl NoteBinding {
    pub(in super::super) fn capture(
        ctx: &Context<'_>,
        expected_input: u8,
        output_pitch: u8,
    ) -> Result<Self, String> {
        let base = ModelBinding::capture_set(ctx)?;
        let mode = base.parameter(ctx, gui_model::Role::LearnMode)?;
        let pitch = base.parameter(ctx, gui_model::Role::InputPitch)?;
        let selected =
            Selection::capture(mode, pitch, expected_input, &mut || base.check_pinned(ctx))?;
        base.check(ctx)?;
        let child = base.capture_note_child(ctx, output_pitch, &mut || {
            base.check_pinned(ctx)?;
            selected.check_borrowed()
        })?;
        let member = child
            .parameter(ctx.api)?
            .pin_raw_toggle_binding("learnChord", &mut || {
                base.check_pinned(ctx)?;
                child.check_borrowed()?;
                selected.check_borrowed()
            })
            .map_err(|error| error.to_string())?;
        let result = Self {
            base,
            pins: NotePins {
                selected,
                child,
                member,
            },
        };
        result.check(ctx)?;
        Ok(result)
    }
    pub(in super::super) fn parameter(&self, ctx: &Context<'_>) -> Result<RawParameter, String> {
        self.check(ctx)?;
        let parameter = self.pins.child.parameter(ctx.api);
        if let Err(reason) = self.check(ctx) {
            release(parameter, &mut || ctx.check())?;
            return Err(reason);
        }
        parameter
    }
    pub(in super::super) fn edit(
        &self,
        parameter: RawParameter,
        enabled: bool,
        ctx: &Context<'_>,
    ) -> parameters::NumericEditAttempt {
        self.pins.edit(
            parameter,
            enabled,
            ctx.api,
            &|| self.base.check(ctx),
            &|| self.base.check_pinned(ctx),
        )
    }
    pub(in super::super) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.pins.check(ctx.api, &|| self.base.check(ctx), &|| {
            self.base.check_pinned(ctx)
        })
    }
    pub(in super::super) fn check_pinned(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.pins.prepare_terminal(&|| self.base.check_pinned(ctx))
    }
}
