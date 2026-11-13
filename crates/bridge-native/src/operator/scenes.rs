//! The walk's Fleet: the page the sketch drew at 13:05, answering each press as Fleet would, and the
//! scenes a step names where Fleet speaks first (the Operator's reply, a new item at 14:02).

use gpui::SharedString;
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorAnswered, OperatorPage, OperatorThen, OperatorTurnItem};
use ipc::operator_talk::{OperatorTalk, OperatorTalkLine};
use ipc::{Instant, WaitingAct};

use super::wire::Failed;


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
    /// How the next answer fails, where a scene wants it refused or its reply lost.
    pub fail_answer: Option<Failed>,
    /// Why the next Later is refused, where a scene wants the time it names to have passed.
    pub refuse_later: Option<String>,
    /// The week's Log, as Fleet writes it.
    pub log: String,
    /// The ideas Fleet keeps for later, oldest first.
    pub parked: Vec<ipc::ParkedIdea>,
    /// Every dispatch it was asked for, for a test to read.
    pub dispatched: Vec<ipc::DispatchJob>,
    /// How many dispatches it has taken, which numbers what it answers with.
    taken: u32,
}

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

/// The ideas Fleet kept before the walk began: three thoughts, one called an idea and one a bug.
pub fn parked() -> Vec<ipc::ParkedIdea> {
    ipc::decode::<ipc::ParkedIdeas>("ParkedIdeas", include_bytes!("../../fixtures/operator-parked.json")).expect("the parked ideas fixture decodes").ideas
}

pub fn talk() -> OperatorTalk {
    ipc::decode::<OperatorTalk>("OperatorTalk", include_bytes!("../../fixtures/operator-talk.json")).expect("the Operator talk fixture decodes")
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
        MockFleet { page: page(), talk: talk(), fronted: 0, heard: Vec::new(), pressed: Vec::new(), refuse_switch: None, fail_answer: None, refuse_later: None, log: LOG.to_string(), parked: parked(), dispatched: Vec::new(), taken: 0 }
    }

    fn take(&mut self, id: &str) -> Option<OperatorTurnItem> {
        let at = self.page.turn.iter().position(|turn| turn.id == id)?;
        Some(self.page.turn.remove(at))
    }

    /// What `operator_answer` does: the item settles, and an act Fleet cannot do itself is left to Bridge,
    /// its item kept on Your turn until he does it, as Fleet keeps it.
    pub fn answer(&mut self, press: &OperatorAnswerPress) -> Result<OperatorAnswered, Failed> {
        self.pressed.push(press.clone());
        if let Some(failed) = self.fail_answer.take() {
            return Err(failed);
        }
        let at = self.page.turn.iter().position(|turn| turn.id == press.id);
        let Some(turn) = self.take(&press.id) else { return Err(Failed::Refused("No open item has that id".into())) };
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
        if answered.open.is_some() {
            self.page.turn.insert(at.unwrap_or(0).min(self.page.turn.len()), turn);
        }
        self.regroup();
        answered.page = self.page.clone();
        Ok(answered)
    }

    /// What `operator_later` does: the item goes last until `until`, or for the rest of the day; a time
    /// already passed is refused, in Fleet's words.
    pub fn later(&mut self, id: &str, until: Option<String>) -> Result<OperatorPage, String> {
        // Fleet's two refusals, word for word (`operator_later_past`).
        let said = until.as_ref().and_then(|until| match chrono::DateTime::parse_from_rfc3339(until) {
            Err(_) => Some("That time could not be read. Pick another."),
            Ok(at) if at <= now() => Some("That time has passed. Pick a later one."),
            Ok(_) => None,
        });
        if let Some(why) = self.refuse_later.take().or_else(|| said.map(String::from)) {
            return Err(why);
        }
        let Some(mut turn) = self.take(id) else { return Err("No open item has that id".into()) };
        turn.later_until = Some(Instant::carried(until.unwrap_or_else(|| "2026-11-11T23:59:59-05:00".into())));
        self.page.turn.push(turn);
        self.regroup();
        Ok(self.page.clone())
    }

    /// What `operator_turn_order` does: the ids it names first, the rest after them as they were, and a
    /// Job's asks together at the place of the first of them.
    pub fn order(&mut self, ids: &[String]) -> OperatorPage {
        let mut rest = std::mem::take(&mut self.page.turn);
        for id in ids {
            if let Some(at) = rest.iter().position(|turn| &turn.id == id) {
                self.page.turn.push(rest.remove(at));
            }
        }
        self.page.turn.append(&mut rest);
        self.regroup();
        self.page.clone()
    }

    /// Fleet's `grouped`: where a Job has two or more asks waiting, or two or more put Later, they share its
    /// id as `group` and stand at the place of the first, in the order it raised them.
    fn regroup(&mut self) {
        let key = |turn: &OperatorTurnItem| turn.session_id.as_ref().map(|id| (turn.later_until.is_some(), id.as_str().to_string()));
        let turn = std::mem::take(&mut self.page.turn);
        let mut placed = std::collections::BTreeSet::new();
        for one in &turn {
            let Some(group) = key(one) else {
                self.page.turn.push(OperatorTurnItem { group: None, ..one.clone() });
                continue;
            };
            if !placed.insert(group.clone()) {
                continue;
            }
            let mut asks: Vec<&OperatorTurnItem> = turn.iter().filter(|other| key(other).as_ref() == Some(&group)).collect();
            asks.sort_by(|a, b| a.raised_at.as_str().cmp(b.raised_at.as_str()));
            let shared = (asks.len() > 1).then(|| group.1.clone());
            self.page.turn.extend(asks.into_iter().map(|ask| OperatorTurnItem { group: shared.clone(), ..ask.clone() }));
        }
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

    /// A new item joins the queue at 14:02: a walk Slot pool controls has for him.
    pub fn arrives(&mut self) {
        let wire = br#"{"id":"turn:5","kind":"decision","session_id":"mock-05","job":"Slot pool controls","manifest_id":"armada","repository":"armada","item":{"id":"walk:armada://walk/slot-meter","text":"Review the slot meter walk","since":"2026-11-11T14:02:00-05:00","source":"walk","act":{"kind":"walk","target":"armada://walk/slot-meter"}},"read":{"read":"The meter holds still when a Job ends, which is what your note asked for.","pick":"Open it"},"raised_at":"2026-11-11T14:02:00-05:00"}"#;
        if let Ok(turn) = ipc::decode::<OperatorTurnItem>("OperatorTurnItem", wire) {
            self.page.turn.push(turn);
        }
    }
}

impl MockFleet {
    /// Three items join at 14:10, each raising a toast: storefront's main gone red with nobody on it, a
    /// walk for him, and a go-ahead to file a note. No Job has stopped.
    pub fn raised(&mut self) {
        let wires: [&[u8]; 3] = [
            br#"{"id":"turn:6","kind":"go_ahead","urgency":"urgent","manifest_id":"storefront","repository":"storefront","item":{"id":"red:storefront:9f2c1e0","text":"storefront's main is red at 9f2c1e0","since":"2026-11-11T14:10:00-05:00","source":"agent"},"read":{"read":"Nobody is on it. desktop_unit fails the way #412 fixed on the branch, so I'd start a Job from that fix.","pick":"Start a Job"},"raised_at":"2026-11-11T14:10:00-05:00"}"#,
            br#"{"id":"turn:7","kind":"decision","urgency":"decision","session_id":"mock-05","job":"Slot pool controls","manifest_id":"armada","repository":"armada","item":{"id":"walk:armada://walk/slot-meter","text":"Review the slot meter walk","since":"2026-11-11T14:11:00-05:00","source":"walk","act":{"kind":"walk","target":"armada://walk/slot-meter"}},"read":{"read":"The meter holds still when a Job ends, which is what your note asked for.","pick":"Open it"},"raised_at":"2026-11-11T14:11:00-05:00"}"#,
            br#"{"id":"turn:8","kind":"go_ahead","urgency":"go_ahead","manifest_id":"armada","repository":"armada","item":{"id":"note:44","text":"the palette forgets its last search","since":"2026-11-11T14:12:00-05:00","source":"agent"},"read":{"read":"No open issue covers it, so I'd file it.","pick":"Just file it"},"raised_at":"2026-11-11T14:12:00-05:00","then":[{"then":"just_file","pick":true}]}"#,
        ];
        for wire in wires {
            if let Ok(turn) = ipc::decode::<OperatorTurnItem>("OperatorTurnItem", wire) {
                self.page.turn.push(turn);
            }
        }
    }
}

impl MockFleet {
    /// What `dispatch_job` does with the Operator on and seated. A dispatch for the words of a kept idea is that idea
    /// being started, and starts. A `feature` or a `bug` starts at once. Anything else, an `idea` or no kind, the
    /// Operator keeps for later. Each answers as Fleet does, with the `operator.dispatched` that follows.
    pub fn dispatch(&mut self, job: ipc::DispatchJob) -> (ipc::JobDispatched, Vec<ipc::operator_board::OperatorDispatched>) {
        use ipc::operator_board::{DispatchWay, OperatorDispatched};
        self.taken += 1;
        let words = job.start.request.clone().unwrap_or_default();
        let title = words.lines().find(|line| !line.trim().is_empty()).map_or("", str::trim).to_string();
        let manifest_id = job.start.manifest_id.as_str().to_string();
        let again = self.parked.iter().any(|idea| idea.dispatch.start.request.as_deref() == Some(words.as_str()));
        let starts = again || matches!(job.kind, Some(ipc::DispatchKind::Feature | ipc::DispatchKind::Bug));
        self.dispatched.push(job.clone());
        if starts {
            let id = format!("composer-{}", self.taken);
            let record = crate::mock::jobs("jobs-many").ok().and_then(|jobs| jobs.into_iter().next()).map(|mut record| {
                record.id = ipc::SessionId::carried(id.clone());
                record.title = Some(title.clone());
                record
            });
            let started = OperatorDispatched { dispatch_id: format!("start:dispatch:{}", self.taken), manifest_id, way: DispatchWay::Started, session_id: Some(ipc::SessionId::carried(id)), title: Some(title) };
            return (ipc::JobDispatched { session: record, operator: None, parked: None }, vec![started]);
        }
        let id = format!("idea:{}", self.taken + 100);
        self.parked.push(ipc::ParkedIdea { id: id.clone(), dispatch: job, at: now().to_rfc3339() });
        let parked = OperatorDispatched { dispatch_id: id.clone(), manifest_id, way: DispatchWay::Parked, session_id: None, title: Some(title) };
        (ipc::JobDispatched { session: None, operator: Some(id), parked: None }, vec![parked])
    }

    /// What `remove_parked_idea` does: the idea goes, and the ones left are answered.
    pub fn remove_parked(&mut self, id: &str) -> Result<ipc::ParkedIdeas, String> {
        let kept = self.parked.len();
        self.parked.retain(|idea| idea.id != id);
        if self.parked.len() == kept {
            return Err("No idea is kept under that id".into());
        }
        Ok(ipc::ParkedIdeas { ideas: self.parked.clone() })
    }
}

impl Default for MockFleet {
    fn default() -> Self {
        MockFleet::new()
    }
}
