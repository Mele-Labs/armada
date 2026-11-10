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
}

pub fn page() -> OperatorPage {
    ipc::decode::<OperatorPage>("OperatorPage", include_bytes!("../../fixtures/operator-page.json")).expect("the Operator page fixture decodes")
}

pub fn talk() -> OperatorTalk {
    ipc::decode::<OperatorTalk>("OperatorTalk", include_bytes!("../../fixtures/operator-talk.json")).expect("the Operator talk fixture decodes")
}

/// The head as the sketch drew it: storefront's main red, armada's green, the week and the slots.
pub fn standing() -> Standing {
    Standing { mains: vec![("storefront".into(), true), ("armada".into(), false)], week: Some(62), slots: Some((7, 10)) }
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
        MockFleet { page: page(), talk: talk(), fronted: 0, heard: Vec::new(), pressed: Vec::new(), refuse_switch: None }
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
