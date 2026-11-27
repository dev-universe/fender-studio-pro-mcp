use std::{
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    bridge::{Bridge, ExchangeFailure, PROTOCOL},
    midi::MidiWakeup,
    model::{HostContext, OperationError, Outcome, Receipt},
    operation::ChorderAudition,
};

const PERMIT_TTL: Duration = Duration::from_secs(15);
const EXCHANGE_LIMIT: Duration = Duration::from_millis(950);
const HOLD: Duration = Duration::from_millis(250);
const NEUTRAL_BITS: &str = "0000000000000000";
const HELD_BITS: &str = "40BF400007D00000";
const CLASS: &str = "D2D9B002-12FC-4F1D-AFDA-70336D1DC108";

struct Target {
    context: HostContext,
    track: String,
    slot: u32,
    instance: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Preflight,
    Observe,
    Press,
    Release,
}

impl Phase {
    const fn name(self) -> &'static str {
        match self {
            Self::Preflight => "preflight",
            Self::Observe => "observe",
            Self::Press => "press",
            Self::Release => "release",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Reply {
    Neutral,
    Held,
    Other,
    Acknowledged,
    Refused,
    Unconfirmed,
}

trait Lane {
    fn context(&mut self) -> Option<HostContext>;
    fn command(&mut self, target: &Target, phase: Phase) -> Reply;
    fn hold(&mut self) {
        thread::sleep(HOLD);
    }
}

pub(crate) struct BridgeLane<'a> {
    pub bridge: &'a Bridge,
    pub midi: &'a mut MidiWakeup,
}

impl Lane for BridgeLane<'_> {
    fn context(&mut self) -> Option<HostContext> {
        let response = self
            .bridge
            .transact_bounded("context", &json!({}), None, None, self.midi, EXCHANGE_LIMIT)
            .ok()?;
        let context: HostContext = serde_json::from_value(response.result).ok()?;
        (response.ok
            && context.open
            && context.protocol == PROTOCOL
            && !context.session.is_empty()
            && !context.document.is_empty()
            && response.session == context.session
            && response.document == context.document)
            .then_some(context)
    }

    fn command(&mut self, target: &Target, phase: Phase) -> Reply {
        match self.bridge.transact_bounded(
            "chorderAuditionLane",
            &json!({"track":target.track,"slot":target.slot,"expected_instance":target.instance,"phase":phase.name()}),
            Some(&target.context.session),
            Some(&target.context.document),
            self.midi,
            EXCHANGE_LIMIT,
        ) {
            Ok(response) if response.ok && response.session == target.context.session
                && response.document == target.context.document => parse_reply(target, phase, &response.result),
            Err(ExchangeFailure { dispatched: false, .. }) => Reply::Refused,
            _ => Reply::Unconfirmed,
        }
    }
}

fn parse_reply(target: &Target, phase: Phase, result: &Value) -> Reply {
    if result["ok"] == false && result["native_outcome"] == "not_applied" {
        return Reply::Refused;
    }
    if result["ok"] != true
        || result["operation"] != "chorder_audition_lane"
        || result["phase"] != phase.name()
        || result["source"] != "dsp"
        || result["instance"]["object_uid"] != target.instance
        || result["instance"]["class_uid"] != CLASS
        || result["instance"]["native_class"] != "MusicEffect"
    {
        return Reply::Unconfirmed;
    }
    match phase {
        Phase::Press | Phase::Release => {
            let expected = if phase == Phase::Press {
                HELD_BITS
            } else {
                NEUTRAL_BITS
            };
            if result["native_outcome"] == "acknowledged" && result["requested_bits"] == expected {
                Reply::Acknowledged
            } else {
                Reply::Unconfirmed
            }
        }
        Phase::Preflight | Phase::Observe if result["native_outcome"] == "observed" => {
            match (
                result["input_bits"].as_str(),
                result["playing_live"].as_bool(),
                result["learn_mode"].as_bool(),
            ) {
                (Some(NEUTRAL_BITS), Some(false), Some(learn))
                    if phase != Phase::Preflight || !learn =>
                {
                    Reply::Neutral
                }
                (Some(HELD_BITS), Some(true), Some(_)) if phase == Phase::Observe => Reply::Held,
                (Some(_), Some(_), Some(_)) => Reply::Other,
                _ => Reply::Unconfirmed,
            }
        }
        _ => Reply::Unconfirmed,
    }
}

#[derive(Default)]
pub(crate) struct Ledger {
    record: Option<Record>,
}

struct Record {
    permit: String,
    target: Target,
    expires: Instant,
    finished: Option<Finished>,
}

struct Finished {
    outcome: Outcome,
    code: Option<&'static str>,
    press_acknowledged: bool,
    held_observed: bool,
    release_attempted: bool,
    release_acknowledged: bool,
    neutral_observed: bool,
    quarantined: bool,
}

impl Finished {
    fn refused(code: &'static str) -> Self {
        Self {
            outcome: Outcome::NotApplied,
            code: Some(code),
            press_acknowledged: false,
            held_observed: false,
            release_attempted: false,
            release_acknowledged: false,
            neutral_observed: false,
            quarantined: false,
        }
    }

    fn unknown(code: &'static str) -> Self {
        Self {
            outcome: Outcome::Unknown,
            quarantined: true,
            ..Self::refused(code)
        }
    }
}

impl Ledger {
    pub(crate) fn handle(
        &mut self,
        action: ChorderAudition,
        expected_document: &str,
        request_id: String,
        started: Instant,
        lane: &mut BridgeLane<'_>,
    ) -> Receipt {
        self.run(action, expected_document, request_id, started, lane)
    }

    fn run(
        &mut self,
        action: ChorderAudition,
        expected_document: &str,
        request_id: String,
        started: Instant,
        lane: &mut impl Lane,
    ) -> Receipt {
        let mut receipt = Receipt {
            request_id,
            outcome: Outcome::NotApplied,
            session: None,
            document: None,
            result: Value::Null,
            error: None,
            elapsed_ms: 0,
        };
        match action {
            ChorderAudition::Prepare {
                track,
                slot,
                expected_instance,
            } => {
                self.prepare(
                    expected_document,
                    track,
                    slot,
                    expected_instance,
                    lane,
                    &mut receipt,
                );
            }
            ChorderAudition::Play { permit } => {
                self.use_permit(&permit, expected_document, false, lane, &mut receipt);
            }
            ChorderAudition::Status { permit } => {
                self.use_permit(&permit, expected_document, true, lane, &mut receipt);
            }
        }
        receipt.elapsed_ms = started.elapsed().as_millis();
        receipt
    }

    fn prepare(
        &mut self,
        expected_document: &str,
        track: String,
        slot: u32,
        instance: String,
        lane: &mut impl Lane,
        receipt: &mut Receipt,
    ) {
        if let Some(previous) = self.record.as_mut()
            && let Some(finished) = previous.finished.as_mut()
            && finished.quarantined
        {
            // This is a read-only fence on the original document and instance.
            // The historical Unknown stays Unknown even after observing neutral.
            if lane.command(&previous.target, Phase::Observe) != Reply::Neutral {
                set_error(receipt, "chorder_audition_quarantined");
                return;
            }
            finished.neutral_observed = true;
            finished.quarantined = false;
        }
        let Some(context) = lane
            .context()
            .filter(|context| context.document == expected_document)
        else {
            set_error(receipt, "chorder_audition_document");
            return;
        };
        let target = Target {
            context,
            track,
            slot,
            instance,
        };
        if lane.command(&target, Phase::Preflight) != Reply::Neutral {
            set_error(receipt, "chorder_audition_preflight");
            return;
        }
        self.record = Some(Record {
            permit: Uuid::new_v4().to_string(),
            target,
            expires: Instant::now() + PERMIT_TTL,
            finished: None,
        });
        self.record.as_ref().unwrap().describe(receipt, false, true);
    }

    fn use_permit(
        &mut self,
        permit: &str,
        expected_document: &str,
        status: bool,
        lane: &mut impl Lane,
        receipt: &mut Receipt,
    ) {
        let Some(record) = self
            .record
            .as_mut()
            .filter(|record| record.permit == permit)
        else {
            set_error(receipt, "chorder_audition_permit");
            return;
        };
        if record.target.context.document != expected_document {
            set_error(receipt, "chorder_audition_document");
            return;
        }
        if status || record.finished.is_some() {
            record.describe(receipt, true, status);
            return;
        }
        if Instant::now() >= record.expires {
            set_error(receipt, "chorder_audition_expired");
            return;
        }
        let expires = std::mem::replace(&mut record.expires, Instant::now());
        record.finished = Some(play(&record.target, expires, lane));
        record.describe(receipt, false, false);
    }
}

impl Record {
    fn describe(&self, receipt: &mut Receipt, historical: bool, observe: bool) {
        receipt.session = Some(self.target.context.session.clone());
        receipt.document = Some(self.target.context.document.clone());
        let mut result = json!({
            "permit": self.permit, "midi_pitch": 60, "historical": historical,
            "scope": "chorder_dsp_local_key_lane",
            "track": self.target.track, "slot": self.target.slot, "instance": self.target.instance,
        });
        if let Some(finished) = &self.finished {
            receipt.outcome = if observe {
                Outcome::Observed
            } else {
                finished.outcome
            };
            if let Some(code) = finished.code.filter(|_| !observe) {
                set_error(receipt, code);
            }
            result["state"] = json!("finished");
            result["recorded_outcome"] = json!(finished.outcome);
            result["reason"] = json!(finished.code);
            result["press_acknowledged"] = json!(finished.press_acknowledged);
            result["held_observed"] = json!(finished.held_observed);
            result["release_attempted"] = json!(finished.release_attempted);
            result["release_acknowledged"] = json!(finished.release_acknowledged);
            result["neutral_observed"] = json!(finished.neutral_observed);
            result["quarantined"] = json!(finished.quarantined);
        } else {
            receipt.outcome = Outcome::Observed;
            let remaining = self.expires.saturating_duration_since(Instant::now());
            result["state"] = json!(if remaining.is_zero() {
                "expired"
            } else {
                "prepared"
            });
            result["expires_in_ms"] = json!(remaining.as_millis());
        }
        receipt.result = result;
    }
}

fn play(target: &Target, expires: Instant, lane: &mut impl Lane) -> Finished {
    if lane.command(target, Phase::Preflight) != Reply::Neutral {
        return Finished::refused("chorder_audition_preflight");
    }
    if Instant::now() >= expires {
        return Finished::refused("chorder_audition_expired");
    }
    match lane.command(target, Phase::Press) {
        Reply::Refused => return Finished::refused("chorder_audition_press_refused"),
        Reply::Acknowledged => {}
        _ => {
            let mut finished = Finished::unknown("chorder_audition_press_unconfirmed");
            let observed = lane.command(target, Phase::Observe);
            finished.held_observed = observed == Reply::Held;
            finished.neutral_observed = observed == Reply::Neutral;
            finished.quarantined = !finished.neutral_observed;
            return finished;
        }
    }

    let observed = lane.command(target, Phase::Observe);
    let mut finished = Finished::unknown("chorder_audition_cycle_unconfirmed");
    finished.press_acknowledged = true;
    if observed == Reply::Neutral {
        finished.neutral_observed = true;
        finished.quarantined = false;
        return finished;
    }
    let held = observed == Reply::Held;
    if held {
        lane.hold();
    }
    // Only a positive press acknowledgement authorizes this one compare-release.
    let released = lane.command(target, Phase::Release) == Reply::Acknowledged;
    let neutral = lane.command(target, Phase::Observe) == Reply::Neutral;
    finished.held_observed = held;
    finished.release_attempted = true;
    finished.release_acknowledged = released;
    finished.neutral_observed = neutral;
    finished.quarantined = !neutral;
    if held && released && neutral {
        finished.outcome = Outcome::AppliedVerified;
        finished.code = None;
    }
    finished
}

fn set_error(receipt: &mut Receipt, code: &'static str) {
    let message = match code {
        "chorder_audition_document" => "The requested document does not match the audition binding",
        "chorder_audition_preflight" => {
            "The bound Chorder lane was not confirmed neutral with transport stopped and Learn off"
        }
        "chorder_audition_permit" => {
            "The permit is unavailable, superseded, or from a previous owner"
        }
        "chorder_audition_expired" => "The audition permit expired",
        "chorder_audition_quarantined" => {
            "The owner must independently observe neutral on the original binding before preparing another audition"
        }
        "chorder_audition_press_refused" => "The guarded native press was refused",
        "chorder_audition_press_unconfirmed" => "The guarded press acknowledgement was unavailable",
        _ => "The owner could not verify the complete press and release cycle",
    };
    receipt.error = Some(OperationError::new(code, message));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    const DOCUMENT: &str = "session:1";
    const INSTANCE: &str = "12345678-1234-1234-1234-123456789ABC";

    struct TestLane {
        replies: VecDeque<(Phase, Reply)>,
        contexts: usize,
        holds: usize,
    }

    impl TestLane {
        fn new(replies: impl IntoIterator<Item = (Phase, Reply)>) -> Self {
            Self {
                replies: replies.into_iter().collect(),
                contexts: 0,
                holds: 0,
            }
        }
    }

    impl Lane for TestLane {
        fn context(&mut self) -> Option<HostContext> {
            self.contexts += 1;
            Some(test_context())
        }

        fn command(&mut self, target: &Target, phase: Phase) -> Reply {
            assert_eq!(target.context.document, DOCUMENT);
            assert_eq!(target.context.session, "session");
            assert_eq!(target.track, "Scratch");
            assert_eq!(target.slot, 1);
            assert_eq!(target.instance, INSTANCE);
            let (expected, reply) = self
                .replies
                .pop_front()
                .expect("unexpected audition command");
            assert_eq!(phase, expected);
            reply
        }

        fn hold(&mut self) {
            self.holds += 1;
        }
    }

    fn test_context() -> HostContext {
        HostContext {
            document: DOCUMENT.into(),
            session: "session".into(),
            protocol: PROTOCOL,
            title: None,
            file_url: None,
            open: true,
        }
    }

    fn run(
        ledger: &mut Ledger,
        lane: &mut TestLane,
        document: &str,
        action: ChorderAudition,
    ) -> Receipt {
        ledger.run(action, document, "test".into(), Instant::now(), lane)
    }

    fn prepare(ledger: &mut Ledger, lane: &mut TestLane) -> Receipt {
        run(
            ledger,
            lane,
            DOCUMENT,
            ChorderAudition::Prepare {
                track: "Scratch".into(),
                slot: 1,
                expected_instance: INSTANCE.into(),
            },
        )
    }

    fn permit(receipt: &Receipt) -> String {
        receipt.result["permit"].as_str().unwrap().to_owned()
    }

    fn action(permit: &str, status: bool) -> ChorderAudition {
        let permit = permit.into();
        if status {
            ChorderAudition::Status { permit }
        } else {
            ChorderAudition::Play { permit }
        }
    }

    #[test]
    fn one_cycle_verifies_independent_states_and_duplicate_returns_history() {
        let mut ledger = Ledger::default();
        let mut lane = TestLane::new([
            (Phase::Preflight, Reply::Neutral),
            (Phase::Preflight, Reply::Neutral),
            (Phase::Press, Reply::Acknowledged),
            (Phase::Observe, Reply::Held),
            (Phase::Release, Reply::Acknowledged),
            (Phase::Observe, Reply::Neutral),
        ]);
        let permit = permit(&prepare(&mut ledger, &mut lane));
        let played = run(&mut ledger, &mut lane, DOCUMENT, action(&permit, false));
        assert_eq!(played.outcome, Outcome::AppliedVerified);
        assert_eq!(played.result["historical"], false);
        assert_eq!(played.result["held_observed"], true);
        assert_eq!(played.result["neutral_observed"], true);
        assert_eq!(lane.holds, 1);
        let replay = run(&mut ledger, &mut lane, DOCUMENT, action(&permit, false));
        assert_eq!(replay.outcome, Outcome::AppliedVerified);
        assert_eq!(replay.result["historical"], true);
        let status = run(&mut ledger, &mut lane, DOCUMENT, action(&permit, true));
        assert_eq!(status.outcome, Outcome::Observed);
        assert_eq!(status.result["recorded_outcome"], "applied_verified");
        assert_eq!(lane.contexts, 1);
        assert!(lane.replies.is_empty());
    }

    #[test]
    fn expired_superseded_and_cross_document_requests_dispatch_no_commands() {
        let mut ledger = Ledger::default();
        let mut lane = TestLane::new([
            (Phase::Preflight, Reply::Neutral),
            (Phase::Preflight, Reply::Neutral),
        ]);
        let superseded = permit(&prepare(&mut ledger, &mut lane));
        let active = permit(&prepare(&mut ledger, &mut lane));
        for (document, action, code) in [
            (
                DOCUMENT,
                action(&superseded, false),
                "chorder_audition_permit",
            ),
            (
                "session:2",
                action(&active, true),
                "chorder_audition_document",
            ),
            (
                "session:2",
                action(&active, false),
                "chorder_audition_document",
            ),
        ] {
            assert_eq!(
                run(&mut ledger, &mut lane, document, action)
                    .error
                    .unwrap()
                    .code,
                code
            );
        }
        ledger.record.as_mut().unwrap().expires = Instant::now();
        assert_eq!(
            run(&mut ledger, &mut lane, DOCUMENT, action(&active, false))
                .error
                .unwrap()
                .code,
            "chorder_audition_expired"
        );
        assert_eq!(
            run(&mut ledger, &mut lane, DOCUMENT, action(&active, true)).result["state"],
            "expired"
        );
        assert_eq!(lane.contexts, 2);
        assert!(lane.replies.is_empty());
    }

    #[test]
    fn lost_press_holds_quarantine_and_never_sends_a_release() {
        let mut ledger = Ledger::default();
        let mut lane = TestLane::new([
            (Phase::Preflight, Reply::Neutral),
            (Phase::Preflight, Reply::Neutral),
            (Phase::Press, Reply::Unconfirmed),
            (Phase::Observe, Reply::Held),
            (Phase::Observe, Reply::Held),
            (Phase::Observe, Reply::Neutral),
            (Phase::Preflight, Reply::Other),
        ]);
        let permit = permit(&prepare(&mut ledger, &mut lane));
        let played = run(&mut ledger, &mut lane, DOCUMENT, action(&permit, false));
        assert_eq!(played.outcome, Outcome::Unknown);
        assert_eq!(played.result["release_attempted"], false);
        assert_eq!(played.result["held_observed"], true);
        assert_eq!(played.result["quarantined"], true);
        assert_eq!(
            prepare(&mut ledger, &mut lane).error.unwrap().code,
            "chorder_audition_quarantined"
        );
        assert_eq!(
            prepare(&mut ledger, &mut lane).error.unwrap().code,
            "chorder_audition_preflight"
        );
        let status = run(&mut ledger, &mut lane, DOCUMENT, action(&permit, true));
        assert_eq!(status.outcome, Outcome::Observed);
        assert_eq!(status.result["recorded_outcome"], "outcome_unknown");
        assert_eq!(status.result["neutral_observed"], true);
        assert_eq!(status.result["quarantined"], false);
        assert_eq!(lane.contexts, 2);
        assert!(lane.replies.is_empty());
    }

    #[test]
    fn lost_release_is_followed_by_a_read_and_remains_unknown_after_neutral() {
        let mut ledger = Ledger::default();
        let mut lane = TestLane::new([
            (Phase::Preflight, Reply::Neutral),
            (Phase::Preflight, Reply::Neutral),
            (Phase::Press, Reply::Acknowledged),
            (Phase::Observe, Reply::Held),
            (Phase::Release, Reply::Unconfirmed),
            (Phase::Observe, Reply::Neutral),
        ]);
        let permit = permit(&prepare(&mut ledger, &mut lane));
        let played = run(&mut ledger, &mut lane, DOCUMENT, action(&permit, false));
        assert_eq!(played.outcome, Outcome::Unknown);
        assert_eq!(played.result["neutral_observed"], true);
        assert_eq!(played.result["release_attempted"], true);
        assert_eq!(played.result["release_acknowledged"], false);
        assert_eq!(played.result["quarantined"], false);
        assert_eq!(lane.holds, 1);
        assert!(lane.replies.is_empty());
    }

    #[test]
    fn acknowledgements_without_independent_held_read_remain_unknown() {
        let mut ledger = Ledger::default();
        let mut lane = TestLane::new([
            (Phase::Preflight, Reply::Neutral),
            (Phase::Preflight, Reply::Neutral),
            (Phase::Press, Reply::Acknowledged),
            (Phase::Observe, Reply::Unconfirmed),
            (Phase::Release, Reply::Acknowledged),
            (Phase::Observe, Reply::Neutral),
        ]);
        let permit = permit(&prepare(&mut ledger, &mut lane));
        let played = run(&mut ledger, &mut lane, DOCUMENT, action(&permit, false));
        assert_eq!(played.outcome, Outcome::Unknown);
        assert_eq!(played.result["held_observed"], false);
        assert_eq!(played.result["neutral_observed"], true);
        assert!(lane.replies.is_empty());
    }

    #[test]
    fn acknowledged_press_that_already_reads_neutral_sends_no_release() {
        let mut ledger = Ledger::default();
        let mut lane = TestLane::new([
            (Phase::Preflight, Reply::Neutral),
            (Phase::Preflight, Reply::Neutral),
            (Phase::Press, Reply::Acknowledged),
            (Phase::Observe, Reply::Neutral),
        ]);
        let permit = permit(&prepare(&mut ledger, &mut lane));
        let played = run(&mut ledger, &mut lane, DOCUMENT, action(&permit, false));
        assert_eq!(played.outcome, Outcome::Unknown);
        assert_eq!(played.result["neutral_observed"], true);
        assert_eq!(played.result["release_attempted"], false);
        assert_eq!(played.result["quarantined"], false);
        assert!(lane.replies.is_empty());
    }

    #[test]
    fn only_exact_typed_native_reply_proves_a_press() {
        let target = Target {
            context: test_context(),
            track: "Scratch".into(),
            slot: 1,
            instance: INSTANCE.into(),
        };
        let mut native = json!({"ok":true,"operation":"chorder_audition_lane","phase":"press","source":"dsp",
            "native_outcome":"acknowledged","requested_bits":HELD_BITS,
            "instance":{"object_uid":INSTANCE,"class_uid":CLASS,"native_class":"MusicEffect"}});
        assert_eq!(
            parse_reply(&target, Phase::Press, &native),
            Reply::Acknowledged
        );
        assert_eq!(
            parse_reply(&target, Phase::Release, &native),
            Reply::Unconfirmed
        );
        native["instance"]["object_uid"] = json!("other");
        assert_eq!(
            parse_reply(&target, Phase::Press, &native),
            Reply::Unconfirmed
        );
    }
}
