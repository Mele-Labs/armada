//! The person's own answers, kept so the Operator can follow one that already covers a new question in the
//! same project, and the settings it reads beside them. `answering.rs` carries a ruling out.
//!
//! **Fleet checks every citation; the call only names one.** An answer is cited as `a<n>` and must be a kept
//! answer of the item's repository or of one in a project with it. A setting is cited as `setting <key> = <value>`
//! and must be a key he saved with that value. Anything else is no citation. **An answer the Operator or
//! Autopilot gave in his place is never kept**, so nothing it did can be followed as if he had said it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::operator_page::{OperatorFollows, OperatorFollowsKind};
use ipc::{Instant, SessionId, SessionRow, SessionVoice, WaitingOption};
use store::KeptAnswer;

use super::raising::{word, ToRaise};
use crate::daemon::Fleet;

/// The file of his newest answers the review call reads.
pub const ANSWERS_FILE: &str = "answers.txt";
/// The file of his settings the review call reads.
pub const SETTINGS_FILE: &str = "settings.txt";
/// The most answers `answers.txt` holds.
pub(super) const ANSWERS_LISTED: usize = 200;
/// How far back a thread is read for the answers it holds, when none are kept yet.
const BACKFILL_DAYS: i64 = 30;

/// What the review call says an item's pick follows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum Follows {
    /// `none`, or nothing readable.
    #[default]
    Nothing,
    /// `a12`.
    Answer(i64),
    /// `setting <key> = <value>`.
    Setting { key: String, value: String },
}

impl Follows {
    /// The `follows:` line of one block, as written.
    pub(super) fn from_words(said: &str) -> Follows {
        let said = said.trim().trim_matches(['"', '.']).trim();
        if let Some(rest) = said.strip_prefix("setting ") {
            return match rest.split_once('=') {
                Some((key, value)) if !key.trim().is_empty() => Follows::Setting { key: key.trim().to_string(), value: value.trim().to_string() },
                _ => Follows::Nothing,
            };
        }
        said.strip_prefix('a').and_then(|n| n.parse::<i64>().ok()).map_or(Follows::Nothing, Follows::Answer)
    }
}

/// A value as it is compared: trimmed, its quotes taken off, runs of white space one space.
fn bare(value: &str) -> String {
    value.trim().trim_matches('"').split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every preference in force, a key and its value each. The two that are long, the layout and the key bindings, are left out.
pub(super) fn preference_pairs(now: &ipc::Preferences) -> Vec<(&'static str, String)> {
    let mut out = vec![
        ("where_things_are_open", now.where_things_are_open.to_string()),
        ("draft_pull_requests", now.draft_pull_requests.to_string()),
        ("theme", now.theme.clone()),
        ("take_in", word(&now.take_in)),
        ("answer_with", word(&now.answer_with)),
        ("read_results", word(&now.read_results)),
        ("look_results", word(&now.look_results)),
        ("look_questions", word(&now.look_questions)),
        ("listen_end_summary", now.listen_end_summary.to_string()),
        ("listen_needs_you", now.listen_needs_you.to_string()),
        ("listen_progress", now.listen_progress.to_string()),
        ("listen_operator", now.listen_operator.to_string()),
        ("buttons_words", now.buttons_words.to_string()),
    ];
    for (key, value) in [("time_format", &now.time_format), ("listen_voice", &now.listen_voice)] {
        if !value.is_empty() {
            out.push((key, value.clone()));
        }
    }
    out
}

/// What a repository's values come to, a key and its word each, where a layer or the default says one.
pub(super) fn value_pairs(values: &ipc::ResolvedValues) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = [
        ("pr_mode", &values.pr_mode),
        ("auto_merge", &values.auto_merge),
        ("review_model", &values.review_model),
        ("theme", &values.theme),
        ("density", &values.density),
    ]
    .into_iter()
    .filter_map(|(key, one)| one.value.clone().map(|value| (key.to_string(), value)))
    .collect();
    for (key, one) in [("agent_starts", &values.agent_starts), ("delivery", &values.delivery)] {
        if let Some(value) = one.as_ref().and_then(|one| one.value.clone()).filter(|value| !value.is_empty()) {
            out.push((key.to_string(), value));
        }
    }
    if let Some(reviews) = &values.reviews {
        out.extend([("reviews_plans", &reviews.plans), ("reviews_pull_requests", &reviews.pull_requests), ("reviews_walks", &reviews.walks)].map(|(key, value)| (key.to_string(), value.clone())));
    }
    out.extend(values.autonomy.iter().map(|one| (format!("autonomy_{}", one.kind), one.value.clone())));
    out
}

/// His choice and his words, as one line.
fn said_of(choice: Option<&str>, words: Option<&str>) -> String {
    [choice, words].into_iter().flatten().collect::<Vec<_>>().join(": ")
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
    /// Keep an answer the person gave to `text`, the question of Job `session`, with the choice he pressed and the
    /// words he wrote. `via` is `job` or `operator_page`. **Never fails what it records**: a warning where it will not write.
    pub(crate) async fn operator_answer_kept(&self, session: &str, text: &str, options: &[WaitingOption], item_id: &str, choice: Option<&str>, words: Option<&str>, via: &str) {
        if choice.is_none() && words.is_none() {
            return;
        }
        let labels: Vec<&str> = options.iter().map(|option| option.label.as_str()).collect();
        let mut store = self.store().lock().await;
        let held = store.hosting(session).ok().flatten().map(|hosting| hosting.manifest_id);
        let kept = store.session(session).ok().flatten();
        let one = KeptAnswer {
            n: 0,
            session: session.to_string(),
            manifest_id: held.or_else(|| kept.as_ref().and_then(|kept| kept.manifest_id.clone())).unwrap_or_default(),
            job: kept.and_then(|kept| kept.title),
            item_id: item_id.to_string(),
            question: text.trim().to_string(),
            options: ipc::encode(&labels).unwrap_or_else(|_| "[]".to_string()),
            choice: choice.map(str::to_string),
            words: words.map(str::to_string),
            answered_at: self.now().as_str().to_string(),
            via: via.to_string(),
        };
        if let Err(why) = store.add_operator_answer(&one) {
            drop(store);
            self.operator_warned("an answer of the person's was not kept", why);
        }
    }

    /// With no answer kept yet, keep the person's `Re: ` messages in the threads of Jobs seen in the last 30 days, once.
    /// The question is the first line after `Re: ` and the rest is his words.
    pub(super) async fn operator_answers_backfilled(&self) {
        let since = self.now().epoch_millis().map(|now| crate::clock::rfc3339_utc(now - BACKFILL_DAYS * 86_400_000));
        let mut store = self.store().lock().await;
        if !store.operator_answers_empty().unwrap_or(false) {
            return;
        }
        let Some(since) = since else { return };
        let found = store.find_sessions(&store::SessionSearch::default()).unwrap_or_default();
        let mut found: Vec<_> = found.into_iter().filter(|session| session.last_seen_at.as_str() >= since.as_str()).collect();
        found.sort_by(|a, b| a.started_at.cmp(&b.started_at));
        let mut kept = Vec::new();
        for session in found {
            let manifest = store.hosting(&session.id).ok().flatten().map(|hosting| hosting.manifest_id).or(session.manifest_id.clone()).unwrap_or_default();
            for body in store.session_rows(&session.id).unwrap_or_default().iter().filter(|body| body.contains("Re: ")) {
                let Ok(SessionRow::Message { id, at, from: SessionVoice::You, text, notice: None, .. }) = ipc::decode::<SessionRow>("a job row", body.as_bytes()) else { continue };
                let Some(rest) = text.strip_prefix("Re: ") else { continue };
                let (question, words) = rest.split_once('\n').map_or((rest, ""), |(first, rest)| (first, rest));
                let words = words.trim();
                if question.trim().is_empty() || words.is_empty() || at.as_str() < since.as_str() {
                    continue;
                }
                kept.push(KeptAnswer {
                    n: 0,
                    session: session.id.clone(),
                    manifest_id: manifest.clone(),
                    job: session.title.clone(),
                    item_id: id,
                    question: question.trim().to_string(),
                    options: "[]".to_string(),
                    choice: None,
                    words: Some(words.to_string()),
                    answered_at: at.as_str().to_string(),
                    via: "thread".to_string(),
                });
            }
        }
        kept.sort_by(|a, b| a.answered_at.cmp(&b.answered_at));
        for one in &kept {
            if let Err(why) = store.add_operator_answer(one) {
                drop(store);
                self.operator_warned("a past answer of the person's was not kept", why);
                return;
            }
        }
    }

    /// `answers.txt`: his newest answers across the repositories Fleet hosts, a block each, starting `a<n>`.
    pub(crate) async fn operator_answers_file(&self) -> String {
        let hosted: Vec<String> = self.repositories().hosted().iter().map(|one| one.manifest().id().as_str().to_string()).collect();
        let kept = self.store().lock().await.operator_answers_of(&hosted, ANSWERS_LISTED).unwrap_or_default();
        let offset = crate::clock::local_offset_secs(self.now().epoch_millis().unwrap_or(0)).await;
        let mut out = String::from("His newest answers to the Jobs' questions, newest first. Follow one only from the project the item is in.\n");
        for one in kept {
            let name = self.board_repository(&one.manifest_id).unwrap_or_default();
            let when = core_model::Timestamp::from_rfc3339(&one.answered_at).epoch_millis().map(|millis| crate::clock::reading_at(millis, offset)).map_or_else(
                || one.answered_at.clone(),
                |at| format!("{:04}-{:02}-{:02} {:02}:{:02}", at.year, at.month, at.day, at.hour, at.minute),
            );
            let options: Vec<String> = ipc::decode::<Vec<String>>("answer options", one.options.as_bytes()).unwrap_or_default().iter().map(|label| format!("\"{label}\"")).collect();
            out.push_str(&format!("\na{}\nProject: {name} (repository id {})\nJob: {}\nWhen: {when}\nQuestion: {}\n", one.n, one.manifest_id, one.job.as_deref().unwrap_or("none"), one.question));
            if !options.is_empty() {
                out.push_str(&format!("Options: {}\n", options.join(", ")));
            }
            out.push_str(&format!("Chose: {}\n", one.choice.as_deref().unwrap_or("none")));
            if let Some(words) = &one.words {
                out.push_str(&format!("Words: {words}\n"));
            }
        }
        out
    }

    /// `settings.txt`: his preferences a `key = value` line each, then each hosted repository's values as
    /// `<repository> <key> = <value>`.
    pub(crate) async fn operator_settings_file(&self) -> String {
        let mut out = String::from("His preferences, a line each.\n");
        for (key, value) in preference_pairs(&self.preferences_in_force()) {
            out.push_str(&format!("{key} = {value}\n"));
        }
        for served in self.repositories().hosted() {
            let id = served.manifest().id().as_str().to_string();
            let name = self.board_repository(&id).unwrap_or_else(|| id.clone());
            out.push_str(&format!("\nThe values of repository {name} (id {id}), as {name} <key> = <value>.\n"));
            if let Ok(values) = self.values_asked(None, Some(ipc::ManifestId::carried(&id))).await {
                for (key, value) in value_pairs(&values) {
                    out.push_str(&format!("{name} {key} = {value}\n"));
                }
            }
        }
        out
    }

    /// The citation of `said` for `one`, where Fleet finds it true. **Anything else is no citation.**
    pub(crate) async fn follows_checked(&self, one: &ToRaise, said: &Follows) -> Option<OperatorFollows> {
        match said {
            Follows::Nothing => None,
            Follows::Answer(n) => {
                let kept = self.store().lock().await.operator_answer(*n).ok().flatten()?;
                let (here, there) = (one.manifest_id.as_str(), kept.manifest_id.as_str());
                if here.is_empty() || there.is_empty() {
                    return None;
                }
                let together = here == there || {
                    let held = self.rule_projects(here);
                    self.rule_projects(there).iter().any(|project| held.contains(project))
                };
                together.then(|| OperatorFollows {
                    kind: OperatorFollowsKind::Answer,
                    session_id: Some(SessionId::carried(&kept.session)),
                    job: kept.job.clone(),
                    question: Some(kept.question.clone()),
                    said: said_of(kept.choice.as_deref(), kept.words.as_deref()),
                    at: Some(Instant::carried(&kept.answered_at)),
                })
            }
            Follows::Setting { key, value } => {
                let mut saved: Vec<(String, String)> = preference_pairs(&self.preferences_in_force()).into_iter().map(|(key, value)| (key.to_string(), value)).collect();
                if !one.manifest_id.is_empty() {
                    if let Ok(values) = self.values_asked(None, Some(ipc::ManifestId::carried(&one.manifest_id))).await {
                        saved.extend(value_pairs(&values));
                    }
                }
                let (_, kept) = saved.into_iter().find(|(held, _)| held == key)?;
                (bare(&kept) == bare(value)).then(|| OperatorFollows { kind: OperatorFollowsKind::Setting, session_id: None, job: None, question: None, said: format!("{key} = {kept}"), at: None })
            }
        }
    }
}
