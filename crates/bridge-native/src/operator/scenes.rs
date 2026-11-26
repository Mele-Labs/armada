//! The walk's Fleet: the page the sketch drew at 13:05, answering each press as Fleet would, and the
//! scenes a step names where Fleet speaks first (the Operator's reply and a card it raises in the talk,
//! a new card at 14:02).

use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorAnswered, OperatorPage, OperatorThen, OperatorTurnItem};
use ipc::{JobThread, OperatorActs, OperatorActsList, WaitingAct};

use super::wire::Failed;

/// What a mock Fleet holds, and what it was told.
#[derive(Clone, Debug)]
pub struct MockFleet {
    pub page: OperatorPage,
    /// The Operator session's thread, the talk's rows.
    pub thread: JobThread,
    /// How often the window said it came to the front.
    pub fronted: usize,
    /// Words sent to the Operator, waiting on a scene to answer them.
    pub heard: Vec<(String, Option<String>)>,
    /// The files sent with the last words heard, which the thread echoes on his row as Fleet's does.
    pub heard_files: Vec<ipc::SentFile>,
    /// Every press, as the routes were asked, for a test to read.
    pub pressed: Vec<OperatorAnswerPress>,
    /// Why the next switch is refused, where a test wants it refused.
    pub refuse_switch: Option<String>,
    /// How the next answer fails, where a scene wants it refused or its reply lost.
    pub fail_answer: Option<Failed>,
    /// Why the next Later is refused, where a scene wants the time it names to have passed.
    pub refuse_later: Option<String>,
    /// Why the next change of what the Operator may do unasked is refused.
    pub refuse_acts: Option<String>,
    /// The Macs waiting for his Confirm to pair, as Fleet's calls carry them.
    pub pairings: Vec<ipc::StandingCall>,
    /// What the Gateway says to the next Confirm or Decline, where a scene wants it refused.
    pub refuse_pairing: Option<String>,
    /// The ideas Fleet keeps for later, oldest first.
    pub parked: Vec<ipc::ParkedIdea>,
    /// Every dispatch it was asked for, for a test to read.
    pub dispatched: Vec<ipc::DispatchJob>,
    /// The Operator's own session, as Fleet's held Jobs carry it: none is a session not started.
    pub session: Option<ipc::SessionRecord>,
    /// The page was emptied by a scene, so switching on again does not bring its cards back.
    pub quiet: bool,
    /// How many dispatches it has taken, which numbers what it answers with.
    taken: u32,
}

pub fn page() -> OperatorPage {
    ipc::decode::<OperatorPage>("OperatorPage", include_bytes!("../../fixtures/operator-page.json")).expect("the Operator page fixture decodes")
}

/// The ideas Fleet kept before the walk began: three thoughts, one called an idea and one a bug.
pub fn parked() -> Vec<ipc::ParkedIdea> {
    ipc::decode::<ipc::ParkedIdeas>("ParkedIdeas", include_bytes!("../../fixtures/operator-parked.json")).expect("the parked ideas fixture decodes").ideas
}

/// The Operator's session record in one of the states the status card draws: `working`, `watching` or `stopped`.
pub fn session(state: &str) -> ipc::SessionRecord {
    let mut all = ipc::decode::<std::collections::BTreeMap<String, ipc::SessionRecord>>("the Operator's sessions", include_bytes!("../../fixtures/operator-session.json")).expect("the Operator session fixture decodes");
    all.remove(state).unwrap_or_else(|| panic!("no Operator session in the state {state}"))
}

/// The Jobs' threads the page's walk opens: the Job view's own, a build to try at the foot of Native Bridge foundation's, and the
/// Operator's session.
pub fn page_threads() -> Result<std::collections::HashMap<String, JobThread>, String> {
    let mut threads = crate::mock::threads("job-view")?;
    let wire = r#"{"kind":"ready_to_try","id":"try:native/strip@5c1e9a2","at":"2026-11-11T14:24:00-05:00","offered_at":"2026-11-11T14:24:00-05:00","branch":"native/strip","tip":"5c1e9a2","about":"The places strip keeps its order after a restart","in_build":true,"state":"open","steps":[{"say":"Drag Kit before Jobs in the strip"},{"say":"Quit and open Bridge again"}]}"#;
    let card = ipc::decode::<ipc::ThreadItem>("ready card", wire.as_bytes()).map_err(|why| why.to_string())?;
    threads.get_mut(READY_JOB).ok_or("no thread for the Job with a build to try")?.items.push(card);
    threads.insert(session("working").id.as_str().to_string(), thread());
    Ok(threads)
}

/// The Job whose build to try comes in front.
const READY_JOB: &str = "mock-02";

/// The Operator session's thread before the walk's talk.
pub fn thread() -> JobThread {
    ipc::decode::<JobThread>("JobThread", include_bytes!("../../fixtures/operator-talk.json")).expect("the Operator talk fixture decodes")
}

/// 13:05 on the sketch's day, the clock the walk's times are read against.
pub fn now() -> chrono::DateTime<chrono::Local> {
    chrono::DateTime::parse_from_rfc3339("2026-11-11T13:05:00-05:00").map(|at| at.with_timezone(&chrono::Local)).unwrap_or_else(|_| chrono::Local::now())
}

/// The Job titles the walk's cards name their Jobs by.
pub fn titles() -> std::collections::HashMap<String, String> {
    crate::mock::jobs_of("jobs-many").unwrap_or_default().into_iter().filter_map(|job| job.title.map(|title| (job.id.as_str().to_string(), title))).collect()
}

fn row(id: &str, at: &str, from: &str, text: &str) -> ipc::ThreadItem {
    let wire = serde_json::json!({ "kind": "row", "row": { "kind": "message", "id": id, "at": at, "from": { "kind": from }, "text": text } });
    ipc::decode::<ipc::ThreadItem>("ThreadItem", wire.to_string().as_bytes()).expect("a talk row decodes")
}

impl MockFleet {
    pub fn new() -> MockFleet {
        MockFleet {
            page: page(),
            // The walk opens on the card: the talk starts with the walk's own words.
            thread: JobThread { items: Vec::new(), ..thread() },
            fronted: 0,
            heard: Vec::new(),
            heard_files: Vec::new(),
            pressed: Vec::new(),
            refuse_switch: None,
            fail_answer: None,
            refuse_later: None,
            refuse_acts: None,
            pairings: Vec::new(),
            refuse_pairing: None,
            parked: parked(),
            dispatched: Vec::new(),
            session: Some(session("working")),
            quiet: false,
            taken: 0,
        }
    }

    fn take(&mut self, id: &str) -> Option<OperatorTurnItem> {
        let at = self.page.turn.iter().position(|turn| turn.id == id)?;
        Some(self.page.turn.remove(at))
    }

    /// What `operator_answer` does: the item settles, and an act Fleet cannot do itself is left to Bridge,
    /// its item kept on Your turn until he does it, as Fleet keeps it. A card settles on the option named.
    pub fn answer(&mut self, press: &OperatorAnswerPress) -> Result<OperatorAnswered, Failed> {
        self.pressed.push(press.clone());
        if let Some(failed) = self.fail_answer.take() {
            return Err(failed);
        }
        let at = self.page.turn.iter().position(|turn| turn.id == press.id);
        let Some(turn) = self.take(&press.id) else { return Err(Failed::Refused("No open item has that id".into())) };
        if let Some(option) = press.option.as_deref().filter(|option| !turn.item.options.iter().any(|one| one.label == *option)) {
            self.page.turn.insert(at.unwrap_or(0).min(self.page.turn.len()), turn);
            return Err(Failed::Refused(format!("{option} is not one of the options")));
        }
        // A card raised in a talk carries it on: settling it settles the card the talk was about.
        if let Some(about) = turn.card.as_ref().and_then(|card| card.talk_of.clone()) {
            self.take(&about);
        }
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
        // A press on a card picks it: it leaves the front for `working` until the Operator settles it.
        let option = press.option.clone().or_else(|| super::face::pick_of(&turn).map(|pick| pick.to_string()));
        match (answered.open.is_some(), option.and_then(|option| super::face::picked_card(&turn, &option, now().to_rfc3339().as_str()))) {
            (false, Some(picked)) => self.page.working.insert(0, picked),
            (true, _) => self.page.turn.insert(at.unwrap_or(0).min(self.page.turn.len()), turn),
            (false, None) => {}
        }
        answered.page = self.page.clone();
        Ok(answered)
    }

    /// What `operator_later` does: the card leaves the queue until `until`, or for the rest of the day, as Fleet leaves a
    /// Later item out of `turn`; a time already passed is refused, in Fleet's words.
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
        if self.take(id).is_none() {
            return Err("No open item has that id".into());
        }
        Ok(self.page.clone())
    }

    /// What saving `features.operator` does: off, nothing is queued.
    pub fn switch(&mut self, on: bool) {
        self.page.on = on;
        if !on {
            self.page.turn.clear();
            self.page.working.clear();
            self.page.on_it = None;
            self.page.top = None;
            // Fleet ends the session when the switch goes off, and the next tick starts one once it is on.
            self.session = None;
        } else if self.page.turn.is_empty() && !self.quiet {
            self.page = page();
        }
    }

    /// The page empties and the Operator is in a turn: nothing needs him, so the status card stands in front.
    pub fn quiet(&mut self) {
        self.quiet = true;
        self.page.turn.clear();
        self.page.working.clear();
        self.session = Some(session("working"));
    }

    /// What `set_operator_acts` does: the project's value is kept, or unset with none; every project's value is answered.
    pub fn set_acts(&mut self, project: &str, acts: Option<OperatorActs>) -> Result<OperatorActsList, String> {
        if let Some(why) = self.refuse_acts.take() {
            return Err(why);
        }
        let Some(one) = self.page.operator_acts.iter_mut().find(|one| one.project == project) else { return Err("No project has that id".into()) };
        one.acts = acts;
        Ok(OperatorActsList { projects: self.page.operator_acts.clone() })
    }

    /// The title of a card on the page, as Fleet's tag line names it.
    fn titled(&self, id: &str) -> String {
        self.page.turn.iter().find(|turn| turn.id == id).map(|turn| turn.item.text.clone()).unwrap_or_default()
    }

    /// The Operator answers the words it was sent last, in its thread, and where they were about a card raises one that
    /// carries on from it: the walk's second card, with its own options. Words about no card get an answer and no card.
    pub fn said(&mut self) -> Option<()> {
        let (words, about) = self.heard.pop()?;
        let n = self.thread.items.len();
        let said = match &about {
            Some(about) => format!("[About card {about} \"{}\"]\n{words}", self.titled(about)),
            None => words,
        };
        let mut his = row(&format!("op-{n:03}"), "2026-11-11T13:09:00-05:00", "you", &said);
        if let ipc::ThreadItem::Row { row: ipc::SessionRow::Message { files, .. } } = &mut his {
            *files = std::mem::take(&mut self.heard_files);
        }
        self.thread.items.push(his);
        let answer = match &about {
            Some(_) => "Pocket's change is small. Holding it for 20 minutes costs less than the conflict. The other way is to move it into Jobs screens, which takes one Job off the files.",
            None => "Pocket, Jobs screens and the desktop_unit fix are running. None of them waits on you.",
        };
        self.thread.items.push(row(&format!("op-{:03}", n + 1), "2026-11-11T13:09:20-05:00", "agent", answer));
        if let Some(about) = about {
            let wire = format!(
                r#"{{"id":"turn:36","kind":"decision","manifest_id":"armada","repository":"armada","item":{{"id":"card:0024","text":"Hold Pocket for 20 minutes or move its change","since":"2026-11-11T13:09:30-05:00","source":"agent","act":{{"kind":"answer","target":"card:0024"}},"options":[{{"label":"Hold Pocket for 20 minutes","description":"Pocket waits"}},{{"label":"Move its change into Jobs screens","description":"Pocket ends"}}]}},"read":{{"read":"You asked about the two Jobs on paging.rs. Both ways avoid the conflict.","pick":"Hold Pocket for 20 minutes"}},"raised_at":"2026-11-11T13:09:30-05:00","card":{{"kind":"raise","talk_of":"{about}","sessions":["mock-05"]}}}}"#
            );
            if let Ok(turn) = ipc::decode::<OperatorTurnItem>("OperatorTurnItem", wire.as_bytes()) {
                self.page.turn.push(turn);
            }
        }
        Some(())
    }

    /// The Operator says more in its thread, with no card raised: what it found on the next tick.
    pub fn replies(&mut self) {
        let n = self.thread.items.len();
        let words = [
            "Pocket pushed at 13:31 and its checks are green. The other Job on paging.rs rebased onto it with no conflict.",
            "Storefront's main is still red. The Job that fixes desktop_unit is in review, and its checks pass.",
            "Nothing else waits on you. I will put a card in front of you when something does.",
        ];
        for (at, said) in words.into_iter().enumerate() {
            self.thread.items.push(row(&format!("op-{:03}", n + at), "2026-11-11T13:40:00-05:00", "agent", said));
        }
    }

    /// The Operator reports the oldest card it is working on failed: the card comes back first in the queue, with its
    /// steps and why.
    pub fn returns(&mut self) -> Option<()> {
        let mut turn = self.page.working.pop()?;
        if let Some(card) = turn.card.as_mut() {
            card.returned = Some(ipc::OperatorCardReturned { why: "The retry Job stopped before it finished".into(), at: ipc::Instant::carried("2026-11-11T13:06:00-05:00") });
        }
        self.page.turn.insert(0, turn);
        Some(())
    }

    /// A Job's plan comes in front at 14:22, and the Operator's session holds its backlog report for him to read.
    pub fn plan_comes(&mut self) -> Option<String> {
        let wire = br#"{"id":"turn:50","kind":"decision","session_id":"mock-01","job":"Fix the merge line freshness chip","manifest_id":"armada","repository":"armada","item":{"id":"plan:2","text":"Plan to approve","since":"2026-11-11T14:22:00-05:00","source":"plan"},"read":{"read":"It keeps the chip's colour in one place and tests it on a stale branch.","pick":"Approve"},"raised_at":"2026-11-11T14:22:00-05:00"}"#;
        let turn = ipc::decode::<OperatorTurnItem>("OperatorTurnItem", wire).ok()?;
        let id = turn.id.clone();
        self.page.turn.insert(0, turn);
        self.session = Some(session("reported"));
        Some(id)
    }

    /// A Job's build to try comes in front at 14:24.
    pub fn build_comes(&mut self) -> Option<String> {
        let wire = br#"{"id":"turn:51","kind":"decision","session_id":"mock-02","job":"Native Bridge foundation","manifest_id":"armada","repository":"armada","item":{"id":"try:mock-02","text":"Ready to try","since":"2026-11-11T14:24:00-05:00","source":"ready_to_try"},"raised_at":"2026-11-11T14:24:00-05:00"}"#;
        let turn = ipc::decode::<OperatorTurnItem>("OperatorTurnItem", wire).ok()?;
        let id = turn.id.clone();
        self.page.turn.insert(0, turn);
        Some(id)
    }

    /// A card comes at 14:20 where nothing was in front.
    pub fn comes(&mut self) {
        let wire = br#"{"id":"turn:41","kind":"decision","manifest_id":"armada","repository":"armada","item":{"id":"card:0029","text":"Checkout's coupon fix has a walk for you","since":"2026-11-11T14:20:00-05:00","source":"agent","act":{"kind":"answer","target":"card:0029"},"options":[{"label":"Open the walk","description":"About 5 min"},{"label":"Approve without it","description":"Its checks are green"}]},"read":{"read":"The total shows once after a coupon, which is what the idea asked for.","pick":"Open the walk"},"raised_at":"2026-11-11T14:20:00-05:00","card":{"kind":"deal","sessions":["mock-05"]}}"#;
        if let Ok(turn) = ipc::decode::<OperatorTurnItem>("OperatorTurnItem", wire) {
            self.page.turn.push(turn);
        }
    }

    /// A new card joins the queue at 14:02, at its end.
    pub fn arrives(&mut self) {
        let wire = br#"{"id":"turn:40","kind":"decision","manifest_id":"armada","repository":"armada","item":{"id":"card:0028","text":"Slot pool controls has a walk for you","since":"2026-11-11T14:02:00-05:00","source":"agent","act":{"kind":"answer","target":"card:0028"},"options":[{"label":"Open the walk","description":"About 5 min"},{"label":"Approve without it","description":"Its checks are green"}]},"read":{"read":"The meter holds still when a Job ends, which is what your note asked for.","pick":"Open the walk"},"raised_at":"2026-11-11T14:02:00-05:00","card":{"kind":"deal","sessions":["mock-05"]}}"#;
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
            let record = crate::mock::jobs_of("jobs-many").ok().and_then(|jobs| jobs.into_iter().next()).map(|mut record| {
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
