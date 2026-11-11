//! The walk's Fleet: the page the sketch drew at 13:05, answering each press as Fleet would, and the
//! scenes a step names where Fleet speaks first (the Operator's reply, a new item at 14:02).

use gpui::SharedString;
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorAnswered, OperatorPage, OperatorThen, OperatorTurnItem};
use ipc::operator_talk::{OperatorTalk, OperatorTalkLine};
use ipc::{Instant, WaitingAct};

use super::face::Standing;

/// What a mock Fleet holds, and what it was told.
#[derive(Clone, Debug)]
pub struct MockFleet {
    pub page: OperatorPage,
    pub talk: OperatorTalk,
    /// How often the window said it came to the front.
    pub fronted: usize,
    /// Words sent to the Operator, waiting on a scene to answer them.
    pub heard: Vec<(String, Option<String>)>,
    /// Every press, as the routes were asked, for a test to read.
    pub pressed: Vec<OperatorAnswerPress>,
    /// Why the next switch is refused, where a test wants it refused.
    pub refuse_switch: Option<String>,
    /// The week's Log, as Fleet writes it.
    pub log: String,
    /// Every repository's Autopilot, as `list_autopilots` answers: none until Away starts one.
    pub autopilots: Vec<ipc::Autopilot>,
    /// What they answered and he has not looked at.
    pub to_review: Vec<ipc::AutopilotDecided>,
    /// Corrections sent on Change, by answer id.
    pub changed: Vec<(String, String)>,
    /// Why the next start is refused, where a test wants it refused.
    pub refuse_away: Option<String>,
}

/// When Away was started in the walk, and when he came back to it.
pub const AWAY_AT: &str = "2026-11-11T13:05:00-05:00";
pub const BACK_AT: &str = "2026-11-11T15:20:00-05:00";

/// The week's Log as Fleet's journal writes it: a heading per day, a line per entry at its time.
const LOG: &str = "# The Operator's journal, 2026-W46

## Tuesday 2026-11-10

- 16:00 Kept a rule for armada: file your notes without asking
- 17:00 Kept a rule for storefront: a red main gets a fix Job at once
- 18:12 Linked: the merge strip report to #3401

## Wednesday 2026-11-11

- 11:24 storefront main went red on 9c41e07; raised it
- 11:31 Started: Fix desktop_unit after #412, for the red main
- 12:18 Ended: every label in Bridge and Pocket is read from the registry
- 12:20 Started: Fix desktop_unit after #412
- 12:33 Passed on: Map legend follow-up, to Rework the Map legend
- 12:40 Held apps/cockpit for Slot pool controls until it pushes
- 12:47 Linked: the build switcher report to #3390
- 13:02 Reading what Store reports its size asks: Review pull request #3415
";

pub fn page() -> OperatorPage {
    ipc::decode::<OperatorPage>("OperatorPage", include_bytes!("../../fixtures/operator-page.json")).expect("the Operator page fixture decodes")
}

pub fn talk() -> OperatorTalk {
    ipc::decode::<OperatorTalk>("OperatorTalk", include_bytes!("../../fixtures/operator-talk.json")).expect("the Operator talk fixture decodes")
}

/// The week as the sketch drew it; the mains and the slots are in the page's fixture.
pub fn standing() -> Standing {
    Standing { week: Some(62) }
}

/// 13:05 on the sketch's day, the clock the walk's times are read against.
pub fn now() -> chrono::DateTime<chrono::Local> {
    chrono::DateTime::parse_from_rfc3339("2026-11-11T13:05:00-05:00").map(|at| at.with_timezone(&chrono::Local)).unwrap_or_else(|_| chrono::Local::now())
}

/// The date he came back, as the sketch has it; he came back at [`now`].
pub fn day() -> SharedString {
    "Wednesday 11 November".into()
}

impl MockFleet {
    pub fn new() -> MockFleet {
        MockFleet { page: page(), talk: talk(), fronted: 0, heard: Vec::new(), pressed: Vec::new(), refuse_switch: None, log: LOG.to_string(), autopilots: Vec::new(), to_review: Vec::new(), changed: Vec::new(), refuse_away: None }
    }

    fn take(&mut self, id: &str) -> Option<OperatorTurnItem> {
        let at = self.page.turn.iter().position(|turn| turn.id == id)?;
        Some(self.page.turn.remove(at))
    }

    /// What `operator_answer` does: the item settles, and an act Fleet cannot do itself is left to Bridge.
    pub fn answer(&mut self, press: &OperatorAnswerPress) -> Result<OperatorAnswered, String> {
        self.pressed.push(press.clone());
        let Some(turn) = self.take(&press.id) else { return Err("No open item has that id".into()) };
        let mut answered = OperatorAnswered::default();
        if press.answer != OperatorAnswer::No {
            let then = press.then.or_else(|| turn.then.iter().find(|offer| offer.pick).map(|offer| offer.then));
            match then {
                Some(OperatorThen::FileAndHand) => {
                    answered.filed = Some("#3421".into());
                    answered.handed_to = turn.then.iter().find_map(|offer| offer.hand_to.as_ref()).map(|to| to.session_id.clone());
                }
                Some(OperatorThen::FileAndStart) => {
                    answered.filed = Some("#3421".into());
                    answered.started = Some(ipc::SessionId::carried("mock-09"));
                }
                Some(OperatorThen::JustFile) => answered.filed = Some("#3421".into()),
                None => answered.open = turn.item.act.clone().filter(|act: &WaitingAct| act.kind != ipc::WaitingActKind::Answer),
            }
        }
        answered.page = self.page.clone();
        Ok(answered)
    }

    /// What `operator_later` does: the item goes last for the rest of the day.
    pub fn later(&mut self, id: &str) -> Result<OperatorPage, String> {
        let Some(mut turn) = self.take(id) else { return Err("No open item has that id".into()) };
        turn.later_until = Some(Instant::carried("2026-11-11T23:59:59-05:00"));
        self.page.turn.push(turn);
        Ok(self.page.clone())
    }

    /// What `operator_turn_order` does: the ids it names first, the rest after them as they were.
    pub fn order(&mut self, ids: &[String]) -> OperatorPage {
        let mut rest = std::mem::take(&mut self.page.turn);
        for id in ids {
            if let Some(at) = rest.iter().position(|turn| &turn.id == id) {
                self.page.turn.push(rest.remove(at));
            }
        }
        self.page.turn.append(&mut rest);
        self.page.clone()
    }

    /// What saving `features.operator` does: off, nothing is queued and nothing is being read.
    pub fn switch(&mut self, on: bool) {
        self.page.on = on;
        if !on {
            self.page.turn.clear();
            self.page.on_it = None;
            self.page.top = None;
        } else if self.page.turn.is_empty() {
            self.page = page();
        }
    }

    /// What `remove_operator_order` does: the rule or hold with that id stops standing.
    pub fn forget(&mut self, id: i64) -> Result<OperatorPage, String> {
        let digest = &mut self.page.digest;
        let (rules, holds) = (digest.learned.len(), digest.holds.len());
        digest.learned.retain(|rule| rule.id != id);
        digest.holds.retain(|hold| hold.id != id);
        if digest.learned.len() == rules && digest.holds.len() == holds {
            return Err("No rule or hold stands with that id".into());
        }
        Ok(self.page.clone())
    }

    /// The Operator answers the words it was sent last.
    pub fn said(&mut self) -> Option<OperatorTalkLine> {
        let (words, about) = self.heard.pop()?;
        let at = "2026-11-11T13:09:00-05:00".to_string();
        self.talk.lines.push(OperatorTalkLine { from_person: true, text: words, at: at.clone(), about: about.clone() });
        let line = OperatorTalkLine {
            from_person: false,
            text: "It holds up the second Job's fix, and the walk can wait an hour.".into(),
            at,
            about,
        };
        self.talk.lines.push(line.clone());
        Some(line)
    }

    /// What `start_autopilot` does: the repository's Autopilot runs on the settings, from 13:05.
    pub fn start_away(&mut self, manifest: &str, settings: ipc::AutopilotSettings) -> Result<ipc::Autopilot, String> {
        if let Some(why) = self.refuse_away.take() {
            return Err(why);
        }
        let autopilot = ipc::Autopilot {
            manifest_id: ipc::ManifestId::carried(manifest),
            instruction: settings.instruction,
            at_once: settings.at_once,
            until: settings.until,
            spend_cap_micros: settings.spend_cap_micros,
            session_cap: settings.session_cap,
            weekly_cap_percent: settings.weekly_cap_percent,
            state: ipc::AutopilotState::Running,
            started_at: Instant::carried(AWAY_AT),
            ended_at: None,
            capped_by: None,
            on_autopilot: settings.on_autopilot,
            decisions: settings.decisions,
            spent_micros: 0,
            sessions_started: 0,
            at_work: 0,
            picks: Vec::new(),
            fault: None,
        };
        self.autopilots.retain(|one| one.manifest_id.as_str() != manifest);
        self.autopilots.push(autopilot.clone());
        Ok(autopilot)
    }

    fn autopilot(&mut self, manifest: &str) -> Result<&mut ipc::Autopilot, String> {
        self.autopilots.iter_mut().find(|one| one.manifest_id.as_str() == manifest).ok_or_else(|| format!("{manifest} has no Autopilot"))
    }

    /// What `stop_autopilot` does: it stops, and what it answered stays to be looked at.
    pub fn stop_away(&mut self, manifest: &str) -> Result<ipc::Autopilot, String> {
        let autopilot = self.autopilot(manifest)?;
        autopilot.state = ipc::AutopilotState::Stopped;
        autopilot.ended_at = Some(Instant::carried(BACK_AT));
        Ok(autopilot.clone())
    }

    /// What `dispatch_set_aside` and `dismiss_set_aside` do to a pick it set aside.
    pub fn pick(&mut self, manifest: &str, pick: &str, start: bool) -> Result<ipc::Autopilot, String> {
        let autopilot = self.autopilot(manifest)?;
        let one = autopilot.picks.iter_mut().find(|one| one.pick_id == pick && one.state == ipc::AutopilotPickState::SetAside).ok_or("That pick is not set aside")?;
        one.state = if start { ipc::AutopilotPickState::Started } else { ipc::AutopilotPickState::Dismissed };
        if start {
            one.session_id = Some(ipc::SessionId::carried("mock-11"));
        }
        Ok(autopilot.clone())
    }

    /// What `review_autopilot_decision` and `override_autopilot_decision` do: the answer leaves the list.
    pub fn reviewed(&mut self, id: &str, change: Option<String>) -> Result<ipc::AutopilotDecisions, String> {
        let before = self.to_review.len();
        self.to_review.retain(|one| one.id != id);
        if self.to_review.len() == before {
            return Err("No answer waits with that id".into());
        }
        if let Some(text) = change {
            self.changed.push((id.to_string(), text));
        }
        Ok(ipc::AutopilotDecisions { to_review: self.to_review.clone() })
    }

    /// While he was away, to 15:20: armada's Autopilot approved Quick dispatch's plan and started a Job
    /// from the backlog, set an issue aside it was unsure of, and storefront's landed the fix's pull request.
    pub fn worked(&mut self) {
        let decided = |id: &str, manifest: &str, job: &str, session: &str, kind, asked: &str, chose: &str, because: &str, at: &str| ipc::AutopilotDecided {
            id: id.into(),
            manifest_id: ipc::ManifestId::carried(manifest),
            session_id: Some(ipc::SessionId::carried(session)),
            who: job.into(),
            kind,
            asked: asked.into(),
            chose: chose.into(),
            because: because.into(),
            at: Instant::carried(at),
        };
        self.to_review = vec![
            decided("mock-01:plan", "armada", "Quick dispatch", "mock-01", ipc::AutopilotDecision::Plan, "Approve plan version 2", "Approve", "Version 2 does what your notes on version 1 asked, and nothing else.", "2026-11-11T13:21:00-05:00"),
            decided("mock-04:pr:412", "storefront", "Fix desktop_unit after #412", "mock-04", ipc::AutopilotDecision::Land, "Approve its pull request", "Land it", "Its checks passed and main is red without it.", "2026-11-11T13:48:00-05:00"),
        ];
        self.page.turn.retain(|turn| !matches!(turn.id.as_str(), "turn:1" | "turn:4"));
        let pick = |id: &str, reference: &str, title: &str, session: Option<&str>, state, reason: &str, at: &str| ipc::AutopilotPick {
            pick_id: id.into(),
            issue_reference: reference.into(),
            issue_title: title.into(),
            issue_url: format!("https://example.invalid/{id}"),
            workflow_id: None,
            session_id: session.map(ipc::SessionId::carried),
            state,
            reason: reason.into(),
            at: Instant::carried(at),
        };
        for autopilot in &mut self.autopilots {
            if autopilot.manifest_id.as_str() == "armada" {
                autopilot.picks = vec![
                    pick("pick-1", "#3398", "Slot meter flickers when a Job ends", Some("mock-10"), ipc::AutopilotPickState::Started, "A small fix with its cause named in the issue.", "2026-11-11T13:34:00-05:00"),
                    pick("pick-2", "#3402", "Move the theme picker", None, ipc::AutopilotPickState::SetAside, "It names two screens, and I could not tell which one you meant.", "2026-11-11T14:40:00-05:00"),
                ];
                autopilot.sessions_started = 1;
                autopilot.spent_micros = 3_200_000;
            }
        }
    }

    /// A new item joins the queue at 14:02: a walk Slot pool controls has for him.
    pub fn arrives(&mut self) {
        let wire = br#"{"id":"turn:5","kind":"decision","session_id":"mock-05","job":"Slot pool controls","manifest_id":"armada","repository":"armada","item":{"id":"walk:armada://walk/slot-meter","text":"Review the slot meter walk","since":"2026-11-11T14:02:00-05:00","source":"walk","act":{"kind":"walk","target":"armada://walk/slot-meter"}},"read":{"read":"The meter holds still when a Job ends, which is what your note asked for.","pick":"Open it"},"raised_at":"2026-11-11T14:02:00-05:00"}"#;
        if let Ok(turn) = ipc::decode::<OperatorTurnItem>("OperatorTurnItem", wire) {
            self.page.turn.push(turn);
        }
    }
}

impl Default for MockFleet {
    fn default() -> Self {
        MockFleet::new()
    }
}
