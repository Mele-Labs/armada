//! Cards Fleet raises to him itself, and answers itself. **Not the Operator's cards**: nothing in the Operator's
//! session raises, revises or settles one, and his press is handled here and never told to it as a pick to carry out.
//!
//! A row of Your turn whose id starts with [`FLEET_CARD`], with no `card` column, so the page draws it as any item with
//! options. What Fleet needs to carry out a press is the [`FleetCardKind`], kept as JSON in the row's `call` column.
//! **A second kind is a variant**: describe it in [`FleetCardKind::described`] and answer it in
//! [`FleetCardKind::answered`]. Nothing else in the page, the press or the store knows kinds.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Instant, WaitingItem, WaitingOption, WaitingSource};
use serde::{Deserialize, Serialize};
use store::KeptTurn;

use super::turning::Fault;
use crate::daemon::Fleet;

/// An item of a card Fleet raised starts with this, then an id of its own.
pub(crate) const FLEET_CARD: &str = "fleet-card:";

/// How long a project's Operator reaches another project after he let it through.
pub(super) const LET_THROUGH_MILLIS: i64 = 60 * 60_000;

/// The picks of a card about another project.
pub(super) const LET_IT_THROUGH: &str = "Let it through";
pub(super) const REFUSE: &str = "Refuse";

/// What Fleet asks him, and what it does with each answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FleetCardKind {
    /// An Operator's call named something in a project that is not its own. `session` is the Operator that asked, `act` the
    /// operation it called and `words` what it asked, in his terms.
    OtherProject { session: String, from_project: String, to_project: String, act: String, words: String },
    /// The router could not tell whose Operator his words are for. `held` is the number of the kept send, `candidates` the
    /// likeliest projects, by id.
    WhichProject { held: i64, candidates: Vec<String> },
}

/// What each project's option on the asking card costs.
const HEARS_THIS: &str = "Its Operator hears this and nothing of the others";

/// The longest stretch of his words the asking card quotes.
const QUOTED: usize = 160;

/// A card as it is drawn: its title, what it reads, and each option with its cost.
pub(crate) struct Described {
    pub(crate) title: String,
    pub(crate) read: String,
    pub(crate) options: Vec<(String, String)>,
    /// The project whose Operator the card is about, which the page tags it with.
    pub(crate) project: Option<String>,
}

/// The sentence for an act, as it follows "wants to".
fn act_said(act: &str) -> &'static str {
    match act {
        "start_another_session" | "dispatch_job" => "start a Job",
        "close_session" => "end a Job",
        "tell_session" => "message a Job",
        "merge_pull_request_by_number" => "merge a pull request",
        "enable_auto_merge" => "set a pull request to merge",
        "list_issues" | "get_issue" | "search_issues" => "read issues",
        "Read" | "Grep" | "Glob" => "read files",
        "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => "change files",
        _ => "act",
    }
}

impl FleetCardKind {
    /// The card he reads for this kind.
    pub(crate) async fn described<H, V, W>(&self, fleet: &Fleet<H, V, W>) -> Described
    where
        H: AgentHarness + Send + Sync + 'static,
        H::Error: std::error::Error + Send + Sync + 'static,
        V: Vcs + Delivery + Send + Sync + 'static,
        V::Error: std::error::Error + Send + Sync + 'static,
        V::CommitError: std::error::Error + Send + Sync + 'static,
        W: WorkProduct + Send + Sync + 'static,
        W::Error: std::error::Error + Send + Sync + 'static,
    {
        match self {
            FleetCardKind::WhichProject { held, candidates } => {
                let words = fleet.held_words(*held).await.unwrap_or_default();
                let quoted: String = words.chars().take(QUOTED).collect();
                Described {
                    title: "Which project is this for?".to_string(),
                    read: format!("He said: \"{quoted}{}\". Nothing in it says whose it is.", if words.chars().count() > QUOTED { "..." } else { "" }),
                    options: candidates.iter().map(|project| (fleet.operator_project_name(project), HEARS_THIS.to_string())).collect(),
                    project: None,
                }
            }
            FleetCardKind::OtherProject { from_project, to_project, act, words, .. } => {
                let (from, to) = (fleet.operator_project_name(from_project), fleet.operator_project_name(to_project));
                let name = fleet.operator_settings_of(from_project).await.name;
                Described {
                    title: format!("{name} ({from}) wants to {} in {to}", act_said(act)),
                    read: format!("It asked: \"{words}\". Nothing was done: {to} is not a project this Operator belongs to."),
                    options: vec![
                        (LET_IT_THROUGH.to_string(), format!("{name} calls again and reaches {to} for one hour")),
                        (format!("Hand it to {to}'s Operator"), format!("{to}'s Operator is told what was asked and decides")),
                        (REFUSE.to_string(), format!("{name} is told no and does not try again")),
                    ],
                    project: Some(from_project.clone()),
                }
            }
        }
    }

    /// Carry out his pick `pick`, or say why it could not be done and leave the card standing.
    pub(super) async fn answered<H, V, W>(&self, fleet: &Arc<Fleet<H, V, W>>, pick: &str) -> Result<(), Fault>
    where
        H: AgentHarness + Send + Sync + 'static,
        H::Error: std::error::Error + Send + Sync + 'static,
        V: Vcs + Delivery + Send + Sync + 'static,
        V::Error: std::error::Error + Send + Sync + 'static,
        V::CommitError: std::error::Error + Send + Sync + 'static,
        W: WorkProduct + Send + Sync + 'static,
        W::Error: std::error::Error + Send + Sync + 'static,
    {
        match self {
            FleetCardKind::WhichProject { held, candidates } => {
                let project = candidates.iter().find(|project| fleet.operator_project_name(project) == pick).ok_or_else(|| Fault::NotOffered(format!("{pick} is not a pick this card offers")))?;
                fleet.held_words_routed(*held, project).await
            }
            FleetCardKind::OtherProject { from_project, to_project, act, words, .. } => {
                let (from, to) = (fleet.operator_project_name(from_project), fleet.operator_project_name(to_project));
                let name = fleet.operator_settings_of(from_project).await.name;
                match pick {
                    LET_IT_THROUGH => {
                        let until = fleet.now().epoch_millis().unwrap_or(0) + LET_THROUGH_MILLIS;
                        fleet.operator_seat().granted(from_project, to_project, until);
                        fleet.operator_told(from_project, &format!("He let `{act}` through to {to} for one hour. Call it again.")).await;
                        Ok(())
                    }
                    REFUSE => {
                        fleet.operator_told(from_project, &format!("He refused `{act}` in {to}. Do not call it again.")).await;
                        Ok(())
                    }
                    handed if handed == format!("Hand it to {to}'s Operator") => {
                        let said = format!("{name}, the Operator of {from}, asked to {} in {to}, and he handed it to you. It asked: \"{words}\". It is yours to decide.", act_said(act));
                        if !fleet.operator_told(to_project, &said).await {
                            return Err(Fault::NotDone(format!("{to}'s Operator is not running, so nothing here would reach it")));
                        }
                        fleet.operator_told(from_project, &format!("He handed `{act}` to {to}'s Operator. Do not call it again.")).await;
                        Ok(())
                    }
                    other => Err(Fault::NotOffered(format!("{other} is not a pick this card offers"))),
                }
            }
        }
    }
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
    /// Raise the card for `kind`, once while it stands: the same kind still open is the card he has not answered.
    /// `None` where it was not kept. **Raised with the Operator off too**, since the page reads nothing then.
    pub(crate) async fn fleet_card_raised(&self, kind: FleetCardKind) -> Option<i64> {
        let Ok(payload) = ipc::encode(&kind) else { return None };
        let drawn = kind.described(self).await;
        let at = self.now();
        let id = format!("{FLEET_CARD}{}", crate::session_host::new_id());
        let item = WaitingItem {
            order: 0,
            why: String::new(),
            approves: false,
            id: id.clone(),
            text: drawn.title,
            since: Instant::carried(at.as_str()),
            source: WaitingSource::OperatorSuggestion,
            act: None,
            options: drawn.options.iter().map(|(label, cost)| WaitingOption { label: label.clone(), description: Some(cost.clone()), picture: None }).collect(),
            steps: Vec::new(),
            place: None,
            confirmed: false,
            holders: Vec::new(),
            creating_since: None,
            create_refused: None,
            operator_read: None,
            artifact: None,
            goal_met: None,
        };
        let row = KeptTurn {
            item: id,
            body: ipc::operator_page::item_kept(&item),
            read_text: Some(drawn.read),
            pick: drawn.options.first().map(|(label, _)| label.clone()),
            raised_at: at.as_str().to_string(),
            call: Some(payload.clone()),
            project: drawn.project,
            ..KeptTurn::default()
        };
        let _turn = self.operator_seat().raising.turn.lock().await;
        let kept = {
            let mut store = self.store().lock().await;
            if store.operator_turn_open().unwrap_or_default().iter().any(|open| open.item.starts_with(FLEET_CARD) && open.call.as_deref() == Some(payload.as_str())) {
                return None;
            }
            store.add_operator_turn(&row)
        };
        match kept {
            Ok(n) => {
                self.operator_page_changed();
                n
            }
            Err(why) => {
                self.operator_warned("a card of Fleet's own was not kept", why);
                None
            }
        }
    }

    /// His press of `pick` on the card `id`: the kind the row kept, answered.
    pub(super) async fn fleet_card_pressed(self: &Arc<Self>, id: &str, pick: &str) -> Result<(), Fault> {
        let row = self.store().lock().await.operator_turn_open().unwrap_or_default().into_iter().find(|row| row.item == id);
        let kind = row
            .and_then(|row| row.call)
            .and_then(|text| ipc::decode::<FleetCardKind>("a card of Fleet's own", text.as_bytes()).ok())
            .ok_or_else(|| Fault::NotDone("Fleet no longer holds what this card asks".to_string()))?;
        kind.answered(self, pick).await
    }

    /// His words kept as send `held`, where it has not been told yet.
    async fn held_words(&self, held: i64) -> Option<String> {
        let kept = self.store().lock().await.operator_held_sends().ok()?.into_iter().find(|(n, _)| *n == held)?.1;
        ipc::decode::<ipc::operator_talk::TalkToOperator>("TalkToOperator", kept.as_bytes()).ok().map(|talk| talk.words)
    }

    /// His pick of `project` for kept send `held`: the send is kept with it, and told to its Operator now where one runs,
    /// else once the session Fleet starts for it is up.
    async fn held_words_routed(self: &Arc<Self>, held: i64, project: &str) -> Result<(), Fault> {
        let gone = || Fault::NotDone("Fleet no longer holds his words".to_string());
        let kept = self.store().lock().await.operator_held_sends().ok().and_then(|sends| sends.into_iter().find(|(n, _)| *n == held)).ok_or_else(gone)?.1;
        let talk = ipc::decode::<ipc::operator_talk::TalkToOperator>("TalkToOperator", kept.as_bytes()).map_err(|_| gone())?;
        let routed = ipc::encode(&ipc::operator_talk::TalkToOperator { project: Some(project.to_string()), ..talk }).map_err(|_| gone())?;
        if !self.operator_on() {
            return Err(Fault::NotDone("the Operator is off, so nothing here would hear it".to_string()));
        }
        if !self.store().lock().await.route_operator_held_send(held, &routed).map_err(|_| gone())? {
            return Err(gone());
        }
        match self.operator_session_id(project).await {
            Some(id) => self.held_sends_delivered(project, &id).await,
            None => self.operator_seat().wake.notify_one(),
        }
        Ok(())
    }

    /// A project as he names it: its name, else its id.
    pub(crate) fn operator_project_name(&self, id: &str) -> String {
        core_model::ScopeId::new(id).ok().and_then(|scope| self.projects_held().get(&scope).map(|one| one.name().as_str().to_string())).unwrap_or_else(|| id.to_string())
    }
}
