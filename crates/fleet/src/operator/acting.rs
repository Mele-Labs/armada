//! What Fleet does on the Operator's ruling, through the ways it already has: Fleet's own word
//! to a live Job (`told_by_fleet`), a note in a thread, a raised call, the person's Start press
//! and the close that parks a Job's slots. **The position of each role is read as a ruling comes
//! back** ([`Acting::position`]), so a call out when it is turned off acts on nothing.

use std::sync::{Arc, Weak};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::settings::{self as keys, Key};
use ipc::operator_positions::{OperatorPosition, OperatorRepositories, OperatorRole};
use ipc::{BeginSession, CloseSession, Notice, SessionId};
use store::settings_file::SettingValue;
use store::KeptPlan;

use super::candidates::shared_paths;
use super::desk::first_words;
use super::{carried, Acted, Acting, Arrival, Choice, Place, Ruling, Way};
use crate::daemon::Fleet;

/// The most of a Job's words one message carries.
const MOST_WORDS: usize = 4_000;

/// Fleet as the Operator's rulings reach it.
pub struct FleetActing<H, V, W> {
    fleet: Weak<Fleet<H, V, W>>,
}

impl<H, V, W> FleetActing<H, V, W> {
    pub fn on(fleet: &Arc<Fleet<H, V, W>>) -> FleetActing<H, V, W> {
        FleetActing { fleet: Arc::downgrade(fleet) }
    }
}

impl<H, V, W> FleetActing<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Run `act` on Fleet, saying how it came out. Where Fleet is gone, nothing was done.
    fn doing<F, Fut>(&self, act: F) -> Acted<'static>
    where
        F: FnOnce(Arc<Fleet<H, V, W>>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        let fleet = self.fleet.upgrade();
        Box::pin(async move {
            match fleet {
                Some(fleet) => act(fleet).await,
                None => Err("Fleet stopped".to_string()),
            }
        })
    }

    /// Run `act` on Fleet in the background, for an act no Log row waits on.
    fn doing_apart<F, Fut>(&self, act: F)
    where
        F: FnOnce(Arc<Fleet<H, V, W>>) -> Fut,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let Some(fleet) = self.fleet.upgrade() else { return };
        tokio::spawn(act(fleet));
    }
}

impl<H, V, W> Acting for FleetActing<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fn position(&self, role: Option<OperatorRole>, manifest: &str) -> OperatorPosition {
        let Some(fleet) = self.fleet.upgrade() else { return OperatorPosition::ReadOnly };
        match role {
            Some(role) => fleet.operator_position(role, manifest),
            None if fleet.operator_on() => OperatorPosition::Decide,
            None => OperatorPosition::ReadOnly,
        }
    }

    fn suggest(&self, arrival: &Arrival, ruling: &Ruling) -> Acted<'_> {
        let (arrival, ruling) = (arrival.clone(), ruling.clone());
        self.doing(move |fleet| async move { fleet.operator_suggested(&arrival, &ruling).await })
    }

    fn go_ahead(&self, session: &str) -> Acted<'_> {
        let session = session.to_string();
        self.doing(move |fleet| async move { fleet.operator_went_ahead(&session).await })
    }

    fn hand_over(&self, session: &str, group: Option<&str>, into: &str, shared: &[String]) -> Acted<'_> {
        let (session, group, into, shared) = (session.to_string(), group.map(str::to_string), into.to_string(), shared.to_vec());
        self.doing(move |fleet| async move { fleet.operator_handed(&session, group.as_deref(), &into, &shared).await })
    }

    fn merge(&self, ending: &str, into: &str, shared: &[String]) -> Acted<'_> {
        let (ending, into, shared) = (ending.to_string(), into.to_string(), shared.to_vec());
        self.doing(move |fleet| async move { fleet.operator_merged(&ending, &into, &shared).await })
    }

    fn keep_in_touch(&self, one: &str, other: &str, shared: &[String]) -> Acted<'_> {
        let (one, other, shared) = (one.to_string(), other.to_string(), shared.to_vec());
        self.doing(move |fleet| async move { fleet.operator_kept_in_touch(&one, &other, &shared).await })
    }

    fn start_rest(&self, session: &str, seen: &str) -> Acted<'_> {
        let (session, seen) = (session.to_string(), seen.to_string());
        self.doing(move |fleet| async move { fleet.operator_started_rest(&session, &seen).await })
    }

    fn raise(&self, session: &str, read: &str) -> Acted<'_> {
        let (session, read) = (session.to_string(), read.to_string());
        self.doing(move |fleet| async move { fleet.operator_raised(&session, &read).await })
    }

    fn place(&self, session: &str, place: &Place) {
        let (session, place) = (session.to_string(), place.clone());
        self.doing_apart(move |fleet| async move { fleet.operator_placed(&session, place).await });
    }

    fn hold(&self, held: &str, first: &str, shared: &[String]) {
        let (held, first, shared) = (held.to_string(), first.to_string(), shared.to_vec());
        self.doing_apart(move |fleet| async move { fleet.operator_held(&held, &first, &shared).await });
    }

    /// Settled as his press would, and the Job's thread says so.
    fn answer(&self, session: &str, item: &str, choice: &Choice, because: &str) -> Acted<'_> {
        let (session, item, choice, because) = (session.to_string(), item.to_string(), choice.clone(), because.to_string());
        self.doing(move |fleet| async move { fleet.operator_answered(&session, &item, &choice, &because).await })
    }

    /// A raise is its Log row.
    fn sourced(&self, arrival: &Arrival, ruling: &Ruling) -> Acted<'_> {
        if ruling.way == Way::Raise {
            return carried();
        }
        let (arrival, ruling) = (arrival.clone(), ruling.clone());
        self.doing(move |fleet| async move { fleet.operator_sourced(&arrival, &ruling).await.map(|_| ()) })
    }
}

/// How much of the work a Job has done, which of two to end is read by: a push first, then files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Done {
    pushed: bool,
    written: usize,
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The Operator's own switch, as settings.json reads now. Off is Fleet as it was before it.
    pub(crate) fn operator_on(&self) -> bool {
        self.machine_settings().now().get(keys::OPERATOR)
    }

    /// The week's plan against the share where the Operator pauses, as the setting and Autopilot's weekly
    /// reading stand now. **No reading is no pause**, and so is a week below the share.
    pub(crate) async fn operator_paused(&self) -> Option<super::Paused> {
        let at = u32::try_from(self.machine_settings().now().get(keys::OPERATOR_PLAN_PAUSE).clamp(0, 100)).unwrap_or(100);
        let week = self.weekly_reading().await?;
        (week >= at).then_some(super::Paused { week, at })
    }

    /// How far `role` goes in the repository `manifest`, as settings.json reads now: Read-only under the
    /// Operator's own switch or a standing order, else what the repository sets, else the machine's.
    pub(crate) fn operator_position(&self, role: OperatorRole, manifest: &str) -> OperatorPosition {
        let settings = self.machine_settings().now();
        if !settings.get(keys::OPERATOR) || self.operator_seat().orders().role_off(role, manifest, &self.now()) {
            return OperatorPosition::ReadOnly;
        }
        let repositories = settings.get(keys::OPERATOR_REPOSITORIES).map(|value| OperatorRepositories::read(&value.to_json_text())).unwrap_or_default();
        repositories.of(manifest, role).unwrap_or_else(|| {
            OperatorPosition::read(&settings.get(match role {
                OperatorRole::Dispatch => keys::OPERATOR_DISPATCH,
                // Dispatch's old switch held Delegate too, so one saved On carries it across.
                OperatorRole::Delegate if settings.saved().get(keys::OPERATOR_DELEGATE.name()).is_none() && settings.saved().get(keys::OPERATOR_DISPATCH.name()) == Some(&SettingValue::Bool(true)) => keys::OPERATOR_DISPATCH,
                OperatorRole::Delegate => keys::OPERATOR_DELEGATE,
                OperatorRole::Answer => keys::OPERATOR_ANSWER,
                OperatorRole::File => keys::OPERATOR_FILE,
            }))
        })
    }

    /// Whether Fleet settles what the Operator answers in `manifest`.
    pub(crate) fn operator_answering(&self, manifest: &str) -> bool {
        self.operator_position(OperatorRole::Answer, manifest) == OperatorPosition::Decide
    }

    /// A Job held on its start card starts as his press would, on the landing the card picked.
    async fn operator_went_ahead(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let Ok((session, hosting)) = self.session_and_hosting(id).await else { return Err("the Job is not there".to_string()) };
        if session.state == store::SessionState::Ended || !hosting.start_held {
            return Err("the Job was no longer waiting on its start".to_string());
        }
        let delivery = self.card_picked(id).await;
        match self.session_begun(BeginSession { session_id: SessionId::carried(id), delivery, message: None }).await {
            Ok(_) => {
                self.fleet_note(id, "Started by the Operator: no live Job shares anything with it.".to_string()).await;
                Ok(())
            }
            Err(why) => {
                self.session_warned(id, "the Operator's start did not begin", format!("{why:?}"));
                Err("the Job did not begin".to_string())
            }
        }
    }

    /// `id`'s work, or one group of its plan, goes to `into`, whose plan already covers it. A whole
    /// Job ends once `into` has taken its words, and not before, so nothing is lost to a mute.
    async fn operator_handed(self: &Arc<Self>, id: &str, group: Option<&str>, into: &str, shared: &[String]) -> Result<(), String> {
        let (title, into_title) = (self.title_of(id).await, self.title_of(into).await);
        let paths = shared.join(", ");
        match group {
            None => {
                let words = self.words_of(id).await;
                let text = format!(
                    "The Operator sent you the work of Job \"{title}\", which ends here: your plan already covers it, \
                     and the two share {paths}. What it was asked:\n\n{words}\n\nRecord your plan again with what it lacks of this."
                );
                if !matches!(self.told_by_fleet(into, &text, Notice::Operator).await, Ok(true)) {
                    return Err(format!("\"{into_title}\" did not take its words"));
                }
                self.fleet_note(id, format!("The Operator sent this Job's work to \"{into_title}\", whose plan already covers it, and ended it.")).await;
                let _ = self.closed_and_parked(CloseSession { session_id: SessionId::carried(id) }).await;
                Ok(())
            }
            Some(group) => {
                let plan = self.store().lock().await.session_plan(id).ok().flatten();
                let Some(shown) = plan.as_ref().and_then(|plan| group_shown(plan, group)) else { return Err("the Job's plan has no such group".to_string()) };
                let text = format!(
                    "The Operator sent you a group of Job \"{title}\"'s plan: your plan already covers it, and the two \
                     share {paths}.\n\n{shown}\n\nRecord your plan again with what it lacks of this."
                );
                if !matches!(self.told_by_fleet(into, &text, Notice::Operator).await, Ok(true)) {
                    return Err(format!("\"{into_title}\" did not take the group"));
                }
                let told = format!(
                    "The Operator sent this group of your plan to Job \"{into_title}\", whose plan already covers it, \
                     sharing {paths}:\n\n{shown}\n\nLeave it to that Job, and record your plan again without it."
                );
                let _ = self.told_by_fleet(id, &told, Notice::Operator).await;
                Ok(())
            }
        }
    }

    /// Two Jobs do the same thing: the one that has done less ends, `ending` where neither has, and
    /// its brief, plan and branch go to the other as one message. **One that has pushed is not
    /// ended**: it is raised to him, since its work is somewhere a close does not reach.
    async fn operator_merged(self: &Arc<Self>, ending: &str, into: &str, shared: &[String]) -> Result<(), String> {
        let (mut ending, mut into) = (ending.to_string(), into.to_string());
        if self.done_of(&into).await < self.done_of(&ending).await {
            std::mem::swap(&mut ending, &mut into);
        }
        let (title, into_title) = (self.title_of(&ending).await, self.title_of(&into).await);
        let paths = shared.join(", ");
        if self.done_of(&ending).await.pushed {
            let text = format!(
                "The Operator finds this Job and \"{into_title}\" doing the same thing, sharing {paths}. This Job has pushed, \
                 so it was not ended: end one of them, or tell them how to split the work."
            );
            self.fleet_note(&ending, text.clone()).await;
            self.call_raised(&ending, Notice::Operator, &text).await;
            return Err("it had pushed, so it was raised to the person and not ended".to_string());
        }
        let words = self.words_of(&ending).await;
        let (plan, branch) = {
            let store = self.store().lock().await;
            (store.session_plan(&ending).ok().flatten(), store.hosting(&ending).ok().flatten().and_then(|one| one.lease_branch))
        };
        let mut text = format!(
            "The Operator merged Job \"{title}\" into this one: the two do the same thing, sharing {paths}. It ends \
             here, and this is what it held. Record your plan again with what it lacks of this.\n\nIts brief:\n\n{words}"
        );
        if let Some(plan) = &plan {
            text.push_str(&format!("\n\nIts plan:\n\n{}", plan_shown(plan)));
        }
        if let Some(branch) = branch {
            text.push_str(&format!("\n\nIts work is committed on branch `{branch}`, not pushed. Take what you need from it."));
        }
        if !matches!(self.told_by_fleet(&into, &text, Notice::Operator).await, Ok(true)) {
            return Err(format!("\"{into_title}\" did not take its words"));
        }
        self.fleet_note(&ending, format!("The Operator merged this Job into \"{into_title}\", which does the same thing, and ended it.")).await;
        let _ = self.closed_and_parked(CloseSession { session_id: SessionId::carried(&ending) }).await;
        Ok(())
    }

    /// Both carry on, each told the other's groups on the shared paths and what the other owns, and
    /// kept so that each is told again when the other moves.
    async fn operator_kept_in_touch(self: &Arc<Self>, one: &str, other: &str, shared: &[String]) -> Result<(), String> {
        let at = self.now().as_str().to_string();
        if self.store().lock().await.keep_operator_in_touch(one, other, shared, &at).is_err() {
            return Err("Fleet could not keep the two in touch".to_string());
        }
        for (to, about) in [(one, other), (other, one)] {
            let text = self.in_touch_said(about, shared, "works on ground you share").await;
            let _ = self.told_by_fleet(to, &format!("{text}\n\nFleet tells you when it changes its plan or pushes on those paths."), Notice::Operator).await;
        }
        Ok(())
    }

    /// What a Job is told of `about`, which `did` something on `shared`.
    pub(super) async fn in_touch_said(&self, about: &str, shared: &[String], did: &str) -> String {
        let title = self.title_of(about).await;
        let plan = self.store().lock().await.session_plan(about).ok().flatten();
        let mut text = format!("Job \"{title}\" {did}: {}.", shared.join(", "));
        match &plan {
            Some(plan) => {
                let on: Vec<String> = plan
                    .groups
                    .iter()
                    .filter(|group| !shared_paths(&group.paths, shared).is_empty())
                    .map(|group| format!("- {} ({})", group.title, group.paths.join(", ")))
                    .collect();
                if !on.is_empty() {
                    text.push_str(&format!(" Its plan's groups there:\n{}", on.join("\n")));
                }
                let owns: Vec<&str> = plan.groups.iter().map(|group| group.title.as_str()).collect();
                text.push_str(&format!("\nIt owns: {}.", owns.join("; ")));
            }
            None => text.push_str(&format!(" It has no plan yet. It was asked: {}", cut(&self.words_of(about).await, 600))),
        }
        text
    }

    pub(super) async fn title_of(&self, id: &str) -> String {
        let session = self.store().lock().await.session(id).ok().flatten();
        session.and_then(|one| one.title).unwrap_or_else(|| id.to_string())
    }

    async fn words_of(&self, id: &str) -> String {
        let words = first_words(&*self.store().lock().await, id);
        cut(&words, MOST_WORDS)
    }

    async fn done_of(&self, id: &str) -> Done {
        let store = self.store().lock().await;
        let pushed = store.session_work(id, "").is_ok_and(|work| work.first_push.is_some());
        Done { pushed, written: store.session_touches(id).map(|touches| touches.len()).unwrap_or(0) }
    }
}

/// A plan as one message carries it: its summary, then each group with its paths and tasks.
fn plan_shown(plan: &KeptPlan) -> String {
    let mut out = plan.summary.trim().to_string();
    for group in &plan.groups {
        out.push_str(&format!("\n\n{}", group_lines(group)));
    }
    out
}

fn group_shown(plan: &KeptPlan, group: &str) -> Option<String> {
    plan.groups.iter().find(|one| one.id == group).map(group_lines)
}

fn group_lines(group: &store::KeptPlanGroup) -> String {
    let mut out = format!("{} ({})", group.title, group.paths.join(", "));
    for task in &group.tasks {
        out.push_str(&format!("\n- {}", task.text));
    }
    out
}

fn cut(text: &str, most: usize) -> String {
    match text.char_indices().nth(most) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_string(),
    }
}
