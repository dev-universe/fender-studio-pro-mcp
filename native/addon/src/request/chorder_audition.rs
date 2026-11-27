use super::*;
use std::ffi::CStr;

const INPUT: &CStr = c"chorder.pitch61.guiInput";
const LIVE: &CStr = c"chorder.pitch61.playingLive";
const LEARN: &CStr = c"chorder.learnMode";
const OFF: u64 = 0;
const ON: u64 = 1.0f64.to_bits();
const HELD: u64 = 0x40BF_4000_07D0_0000;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Preflight,
    Observe,
    Press,
    Release,
}

struct LaneParameter {
    name: &'static CStr,
    tag: i32,
    kind: parameters::Kind,
    max: f64,
    read_only: bool,
}

const KEY_INPUT: LaneParameter = LaneParameter {
    name: INPUT,
    tag: 433,
    kind: parameters::Kind::Float,
    max: 10_000.0,
    read_only: false,
};
const PLAYING_LIVE: LaneParameter = LaneParameter {
    name: LIVE,
    tag: 435,
    kind: parameters::Kind::Toggle,
    max: 1.0,
    read_only: true,
};
const LEARN_MODE: LaneParameter = LaneParameter {
    name: LEARN,
    tag: 6,
    kind: parameters::Kind::Toggle,
    max: 1.0,
    read_only: false,
};

impl LaneParameter {
    fn bind(
        &self,
        ctx: &Context<'_>,
        controller: &Controller,
    ) -> Result<(RawParameter, parameters::Description), String> {
        let name = self.name.to_str().expect("ASCII Chorder parameter name");
        let parameter = controller.find(name, ctx.api)?;
        let description = parameter.describe().map_err(|error| error.to_string())?;
        let bits = |value: Option<&parameters::NativeValue>| match value {
            Some(parameters::NativeValue {
                variant_tag: 2,
                value: parameters::Value::Float(value),
            }) => Some(value.to_bits()),
            _ => None,
        };
        if description.name != name
            || description.tag != self.tag
            || description.kind != self.kind
            || !description.enabled
            || description.read_only != self.read_only
            || bits(description.min.as_ref()) != Some(OFF)
            || bits(description.max.as_ref()) != Some(self.max.to_bits())
            || bits(description.raw.as_ref()).is_none()
            || !self.attached(controller, &parameter)
        {
            return Err("The exact Chorder DSP lane metadata was unavailable".into());
        }
        Ok((parameter, description))
    }

    fn attached(&self, controller: &Controller, parameter: &RawParameter) -> bool {
        controller.find_borrowed(self.name) == parameter.as_borrowed_raw().as_ptr()
    }
}

pub(super) fn execute(ctx: &Context<'_>, provider: &Provider, phase: Phase) -> Value {
    if ctx.instance.class_uid != chorder_chords::CLASS || ctx.instance.native_class != "MusicEffect"
    {
        return error(
            "chorder_audition_class",
            "Exact native Chorder MusicEffect required",
            "not_applied",
        );
    }
    let Provider::DspValue(controller) = provider else {
        return error(
            "chorder_audition_provider",
            "Exact Chorder DSP controller required",
            "not_applied",
        );
    };
    run(ctx, controller, phase)
        .unwrap_or_else(|reason| error("chorder_audition_lane", &reason, "not_applied"))
}

fn run(ctx: &Context<'_>, controller: &Controller, phase: Phase) -> Result<Value, String> {
    ctx.check()?;
    let (input, input_header) = KEY_INPUT.bind(ctx, controller)?;
    let (playing, playing_header) = PLAYING_LIVE.bind(ctx, controller)?;
    let (learn, learn_header) = LEARN_MODE.bind(ctx, controller)?;
    let mut guard = || {
        controller.check()?;
        ctx.check()?;
        if !KEY_INPUT.attached(controller, &input)
            || !PLAYING_LIVE.attached(controller, &playing)
            || !LEARN_MODE.attached(controller, &learn)
            || input.check_structure_header(&input_header).is_err()
            || playing.check_structure_header(&playing_header).is_err()
            || learn.check_structure_header(&learn_header).is_err()
        {
            return Err("The bound Chorder DSP parameter was replaced".into());
        }
        ctx.check()
    };
    let learn_value = learn.pin_float().map_err(|error| error.to_string())?;
    let playing_value = playing.pin_float().map_err(|error| error.to_string())?;
    if matches!(phase, Phase::Observe | Phase::Preflight) {
        let input_value = input.pin_float().map_err(|error| error.to_string())?;
        guard()?;
        let bits = input_value.bits().map_err(|error| error.to_string())?;
        let playing_live = toggle(&playing_value)?;
        let learn_mode = toggle(&learn_value)?;
        guard()?;
        return Ok(
            json!({"ok":true,"outcome":"observed","operation":"chorder_audition_lane",
            "phase":phase,"input_bits":format!("{bits:016X}"),"playing_live":playing_live,"learn_mode":learn_mode}),
        );
    }
    let (expected, desired) = match phase {
        Phase::Press => (OFF, HELD),
        Phase::Release => (HELD, OFF),
        _ => unreachable!("read phases returned above"),
    };
    let mut admit = || {
        if phase == Phase::Press && (toggle(&learn_value)? || toggle(&playing_value)?) {
            return Err("Chorder Learn or the local live key is already active".into());
        }
        Ok(())
    };
    match input.set_raw_float_if(expected, f64::from_bits(desired), &mut guard, &mut admit) {
        Ok(_) => Ok(
            json!({"ok":true,"outcome":"acknowledged","operation":"chorder_audition_lane",
            "phase":phase,"requested_bits":format!("{desired:016X}")}),
        ),
        Err(failed) => {
            let outcome = match failed.outcome {
                parameters::Outcome::NotApplied => "not_applied",
                parameters::Outcome::Unknown => "unknown",
            };
            Ok(error(
                "chorder_audition_write",
                &failed.error.to_string(),
                outcome,
            ))
        }
    }
}

fn toggle(value: &parameters::FloatWitness<'_>) -> Result<bool, String> {
    match value.bits().map_err(|error| error.to_string())? {
        OFF => Ok(false),
        ON => Ok(true),
        _ => Err("The Chorder toggle did not have a canonical Float carrier".into()),
    }
}
