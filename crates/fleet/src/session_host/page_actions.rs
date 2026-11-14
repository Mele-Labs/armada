//! A page's actions: buttons the agent declares with `<armada-action>`, and `press_page_action`, his press of one.
//! **Read off the kept revision he pressed, never off a call.** Fleet keeps the list of acts a page may run, start,
//! answer and message, and refuses anything off it: anything that lands, merges, deletes or stops goes to the Job
//! as a message. `crates/ipc/operations/press_page_action.toml` is the contract, `docs/concepts/artifact.md`,
//! *Page actions*, the rules.

use std::collections::BTreeSet;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{
    AnswerWaiting, ArtifactForm, DispatchJob, DispatchMessage, DispatchThen, Instant, ManifestId, PageAct, PagePress, PressDid, PressPageAction,
    SendSessionMessage, SessionId, StartSession,
};
use store::KeptPress;

use super::document_page::{actions_in, PageAction};
use super::documents::revisions_named;
use crate::daemon::Fleet;

const PAGE_ACTION_UNKNOWN: &str = "fleet.page_action_unknown";
const PAGE_ACT_REFUSED: &str = "fleet.page_act_refused";
const PAGE_ASK_NOT_OPEN: &str = "fleet.page_ask_not_open";
/// An archived page takes no press and no vote.
pub(super) const PAGE_ARCHIVED: &str = "fleet.page_archived";
/// Why an act off the list is refused, at the press.
const OFF_THE_LIST: &str = "a page runs start, answer and message only. Anything that lands, merges, deletes or stops goes to the Job as a message";
/// The most action ids a refusal names.
const MOST_NAMED: usize = 20;

/// The one line that tells a Job which actions of its page will not run, or `None` where all will. The page is
/// never refused.
pub(crate) fn actions_that_will_not_run(html: &str, path: &str) -> Option<String> {
    let mut seen = BTreeSet::new();
    let failing: Vec<String> = actions_in(html)
        .iter()
        .enumerate()
        .filter_map(|(at, action)| {
            let why = if action.id.is_empty() {
                "it has no id".to_string()
            } else if !seen.insert(action.id.clone()) {
                format!("its id \"{}\" is an earlier action's too", action.id)
            } else if action.act.is_empty() {
                "it has no act. A page runs start, answer and message".to_string()
            } else if PageAct::named(&action.act).is_none() {
                format!(
                    "act \"{}\" is not one a page runs. A page runs start, answer and message only, and anything that lands, merges, deletes or stops is a message button whose label says what the Job will do",
                    action.act
                )
            } else {
                return None;
            };
            Some(format!("In the page {path}, action \"{}\" will not run: {why}.", named(action, at)))
        })
        .collect();
    (!failing.is_empty()).then(|| failing.join(" "))
}

/// An action as a notice names it: its label, else its id, else its place on the page.
fn named(action: &PageAction, at: usize) -> String {
    [&action.label, &action.id].into_iter().find(|one| !one.is_empty()).cloned().unwrap_or_else(|| format!("number {}", at + 1))
}

/// `on <item>`, or nothing where the action names no item.
fn on_item(item: &str) -> String {
    match item.is_empty() {
        true => String::new(),
        false => format!(" on {item}"),
    }
}

/// What the Job is sent for a `message` press: what he pressed, the row's text, then his words.
fn pressed_told(action: &PageAction, title: &str, words: Option<&str>) -> String {
    let mut lines = vec![format!("He pressed \"{}\"{} in \"{title}\".", action.label, on_item(&action.item))];
    lines.extend([action.row.as_str(), words.unwrap_or_default()].into_iter().filter(|one| !one.is_empty()).map(str::to_string));
    lines.join("\n")
}

/// The brief a `start` press starts a Job on: the action's brief, where it came from, then his words.
fn started_brief(action: &PageAction, title: &str, words: Option<&str>) -> String {
    let mut from = format!("From \"{title}\"");
    if !action.item.is_empty() {
        from.push_str(&format!(", {}", action.item));
    }
    match action.row.is_empty() {
        true => from.push('.'),
        false => from.push_str(&format!(": {}", action.row)),
    }
    [action.brief.as_str(), from.as_str(), words.unwrap_or_default()].into_iter().filter(|one| !one.is_empty()).collect::<Vec<_>>().join("\n\n")
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
    /// `press_page_action`: the action read off the kept revision, run, and the press kept.
    pub(crate) async fn page_action_pressed(self: Arc<Self>, press: PressPageAction) -> Result<PagePress, Refusal> {
        let id = press.session_id.as_str().trim().to_string();
        let artifact = self.artifact_held(&id, &press.artifact).await?;
        if artifact.form != ArtifactForm::Page {
            return Err(self.hosted_refusal(PAGE_ACTION_UNKNOWN, &format!("`{}` is not a page, and only a page declares actions", artifact.title)));
        }
        if artifact.archived.is_some() {
            return Err(self.hosted_refusal(PAGE_ARCHIVED, &format!("`{}` is archived. Restore it to press its actions", artifact.title)));
        }
        let action = self.action_kept(&id, &artifact.id, &artifact.title, press.revision, &press.action).await?;
        let Some(act) = PageAct::named(&action.act) else {
            return Err(self.hosted_refusal(PAGE_ACT_REFUSED, OFF_THE_LIST));
        };
        let words = press.words.as_deref().map(str::trim).filter(|words| !words.is_empty());
        let did = match act {
            PageAct::Message => {
                self.told_as_him(&id, pressed_told(&action, &artifact.title, words)).await?;
                PressDid::Sent
            }
            PageAct::Start => {
                let job = Arc::clone(&self).started_from_page(&id, &action, &artifact.title, words).await?;
                let short: String = job.as_str().chars().take(8).collect();
                // The Job exists whatever this send does, so the press is kept either way.
                let told = format!("He started Job {short} from \"{}\"{}.", action.label, on_item(&action.item));
                let _ = self.told_as_him(&id, told).await;
                PressDid::Started { job }
            }
            PageAct::Answer => {
                let item = self.ask_answered(&id, &action, words).await?;
                PressDid::Answered { item }
            }
        };
        let item = Some(action.item.clone()).filter(|item| !item.is_empty());
        let at = self.instant();
        let (did_word, target) = match &did {
            PressDid::Sent => ("sent", None),
            PressDid::Started { job } => ("started", Some(job.as_str().to_string())),
            PressDid::Answered { item } => ("answered", Some(item.clone())),
        };
        let kept = KeptPress {
            artifact: artifact.id.clone(),
            revision: press.revision,
            action: action.id.clone(),
            item: item.clone(),
            act: act.word().to_string(),
            at: at.as_str().to_string(),
            did: did_word.to_string(),
            target,
        };
        self.store().lock().await.keep_page_press(&id, &kept).map_err(|why| self.ledger_fault(why))?;
        self.published_hosted(&id).await?;
        Ok(PagePress { revision: press.revision, action: action.id, item, act, at, did })
    }

    /// The first action with id `named` in revision `revision` of the page at `path`, as Fleet kept it.
    async fn action_kept(&self, id: &str, path: &str, title: &str, revision: u32, named: &str) -> Result<PageAction, Refusal> {
        let (_, held) = self.document_held(id, path).await?;
        let file = held.map(|page| revisions_named(&page.detail)).and_then(|kept| kept.get((revision as usize).checked_sub(1)?).cloned());
        let Some(file) = file else {
            return Err(self.hosted_refusal(PAGE_ACTION_UNKNOWN, &format!("revision {revision} of `{title}` is not kept")));
        };
        let html = tokio::task::spawn_blocking(move || std::fs::read_to_string(file)).await.ok().and_then(Result::ok).unwrap_or_default();
        let actions = actions_in(&html);
        let named = named.trim();
        if let Some(action) = actions.iter().find(|one| !named.is_empty() && one.id == named) {
            return Ok(action.clone());
        }
        let ids: Vec<&str> = actions.iter().map(|one| one.id.as_str()).filter(|one| !one.is_empty()).take(MOST_NAMED).collect();
        let holds = match ids.is_empty() {
            true => String::from("none"),
            false => ids.join(", "),
        };
        Err(self.hosted_refusal(PAGE_ACTION_UNKNOWN, &format!("revision {revision} of `{title}` holds no action `{named}`. Actions it holds: {holds}")))
    }

    /// A Job started at once, as his own start from the dispatch sheet: unheld, the brief its request and its first
    /// message, and past the Operator, since the button is his press. The new Job's id.
    async fn started_from_page(self: Arc<Self>, id: &str, action: &PageAction, title: &str, words: Option<&str>) -> Result<SessionId, Refusal> {
        let (_, hosting) = self.session_and_hosting(id).await?;
        let own = action.repository.is_empty() || action.repository == hosting.manifest_id;
        let manifest_id = match own {
            true => hosting.manifest_id.clone(),
            false => action.repository.clone(),
        };
        let brief = started_brief(action, title, words);
        let start = StartSession {
            manifest_id: ManifestId::carried(&manifest_id),
            title: None,
            model: None,
            effort: None,
            mode: None,
            fork: None,
            root: None,
            request: Some(brief.clone()),
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
            // The page's Job's project, where the new Job is in its repository.
            project: hosting.project.clone().filter(|_| own),
        };
        let then = vec![DispatchThen::Message(DispatchMessage { text: brief, attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() })];
        let record = self.dispatch_started(DispatchJob { start, kind: None, then }, true).await?;
        Ok(record.id)
    }

    /// The Job's ask the action names, answered with its option as his own pick answers it. The ask's id.
    async fn ask_answered(self: &Arc<Self>, id: &str, action: &PageAction, words: Option<&str>) -> Result<String, Refusal> {
        let open = self.waiting_now(id).await?.into_iter().find(|item| !action.ask.is_empty() && item.id == action.ask);
        let Some(item) = open else {
            return Err(self.hosted_refusal(PAGE_ASK_NOT_OPEN, &format!("the Job no longer waits on `{}`", action.ask)));
        };
        let Some(choice) = item.options.iter().position(|option| option.label.trim() == action.option) else {
            let labels: Vec<&str> = item.options.iter().map(|option| option.label.as_str()).collect();
            let why = format!("\"{}\" is not one of the options of `{}`: {}", action.option, item.id, labels.join(", "));
            return Err(self.hosted_refusal(PAGE_ASK_NOT_OPEN, &why));
        };
        let said = AnswerWaiting {
            session_id: SessionId::carried(id),
            item_id: item.id.clone(),
            choice: Some(choice as u32),
            text: words.map(str::to_string),
            mode: None,
            fixed: None,
        };
        Arc::clone(self).answer_waiting(said).await?;
        Ok(item.id)
    }

    /// Send the Job `text` as his message.
    pub(super) async fn told_as_him(self: &Arc<Self>, id: &str, text: String) -> Result<(), Refusal> {
        let terminal = self.terminal_session(id).await?;
        let told = SendSessionMessage { session_id: SessionId::carried(id), text, attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() };
        Arc::clone(self).send_known(terminal, told).await.map(drop)
    }
}

/// The presses kept for a page, as the wire reads them. One whose act or result no longer reads is left out.
pub(crate) fn presses_of(kept: &[KeptPress], artifact: &str) -> Vec<PagePress> {
    kept.iter()
        .filter(|one| one.artifact == artifact)
        .filter_map(|one| {
            let did = match (one.did.as_str(), one.target.clone()) {
                ("sent", _) => PressDid::Sent,
                ("started", Some(job)) => PressDid::Started { job: SessionId::carried(&job) },
                ("answered", Some(item)) => PressDid::Answered { item },
                _ => return None,
            };
            Some(PagePress {
                revision: one.revision,
                action: one.action.clone(),
                item: one.item.clone(),
                act: PageAct::named(&one.act)?,
                at: Instant::carried(one.at.clone()),
                did,
            })
        })
        .collect()
}
