//! The Operator's cards: what its own session puts in front of the person, one decision each, and what
//! his press, his Later and his talk say back to it. `crates/ipc/operations/raise_card.toml`.
//!
//! **A card is a row of Your turn** that no Job's list holds: `card:<id>` in the queue the page, the
//! press and Later already serve, with its options as the item's and its cost as each one's description.
//! Only the Operator's own session raises, revises, reports on or settles one, placed by its connection. **His acts
//! reach it as messages** carrying the card's id and title, and a press while the session is not
//! running is refused, since nobody would hear it.
//!
//! **His press picks the card**: it leaves the front for the page's `working` until the session settles it, and
//! comes back first in Your turn where the session reports it failed or nothing settles it in [`STALL_MILLIS`].

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::operator_page::item_kept;
use ipc::operator_page::OperatorFeedKind;
use ipc::operator_talk::{OperatorSaid, OperatorTalk, OperatorTalkLine, TalkToOperator};
use ipc::{SessionTag, SessionUpload, CardOption, CardRaised, CardRevised, OperatorCardSettled, Instant, OperatorCard, RaiseCard, ReportCard, ReviseCard, SettleCard, WaitingAct, WaitingActKind, WaitingItem, WaitingOption, WaitingSource};
use store::KeptTurn;

use super::turning::turn_number;
use crate::daemon::Fleet;

/// What a card's item id begins with, then an id of its own.
pub(crate) const CARD: &str = "card:";
/// What a card its Operator settled is answered with, then what it said.
pub(crate) const SETTLED: &str = "settled:";

const CARD_REFUSED: &str = "fleet.card_refused";
const CARD_UNKNOWN: &str = "fleet.card_unknown";

/// The most lines of the earlier conversation the page reads.
const TALK_SHOWN: u32 = 20;

/// How long a picked card waits on its settle before it comes back to the front.
pub(crate) const STALL_MILLIS: i64 = 15 * 60_000;
/// Why a picked card nothing settled came back.
pub(crate) const STALLED: &str = "Nothing settled in 15 min";

/// The fewest and most options a card holds.
const OPTIONS: std::ops::RangeInclusive<usize> = 2..=4;

/// The options of a card as the item draws them: a label, and its cost under it.
fn options_of(options: &[CardOption]) -> Vec<WaitingOption> {
    options.iter().map(|one| WaitingOption { label: one.label.trim().to_string(), description: Some(one.cost.trim().to_string()), picture: None }).collect()
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
    fn card_refused(&self, why: &str) -> Refusal {
        self.hosted_refusal(CARD_REFUSED, why)
    }

    /// Each option checked: two to four, each with a label and a cost, no label twice.
    fn card_options_checked(&self, options: &[CardOption]) -> Result<(), Refusal> {
        if !OPTIONS.contains(&options.len()) {
            return Err(self.card_refused(&format!("a card has two to four options, and this one has {}", options.len())));
        }
        for (at, one) in options.iter().enumerate() {
            if one.label.trim().is_empty() || one.cost.trim().is_empty() {
                return Err(self.card_refused(&format!("option {} has no label or no cost. Each option says what it is and what it spends or risks", at + 1)));
            }
            if options[..at].iter().any(|before| before.label.trim() == one.label.trim()) {
                return Err(self.card_refused(&format!("two options are called {}", one.label.trim())));
            }
        }
        Ok(())
    }

    /// The repository named, as its manifest's id: its folder's name, or the id itself.
    fn repository_id_of(&self, name: &str) -> Option<String> {
        self.repositories().hosted().into_iter().find_map(|one| {
            let id = one.manifest().id().as_str().to_string();
            let folder = std::path::Path::new(one.root()).file_name().map(|folder| folder.to_string_lossy().to_string());
            (id == name || folder.as_deref() == Some(name)).then_some(id)
        })
    }

    /// The open card `id`, `turn:<n>`: not found where it is not there, settled, or no card.
    async fn card_open(&self, id: &str) -> Result<KeptTurn, Refusal> {
        let unknown = || Refusal::NotFound(ipc::WireError::raised(CARD_UNKNOWN, format!("{id} is not a card waiting in Your turn"), self.run_id()));
        let n = turn_number(id).ok_or_else(unknown)?;
        let row = self.store().lock().await.operator_turn_item(n).map_err(|why| self.ledger_fault(why))?;
        row.filter(|row| row.settled_at.is_none() && row.card.is_some()).ok_or_else(unknown)
    }

    /// `raise_card`.
    pub(crate) async fn card_raised(self: &Arc<Self>, caller: Option<api::Caller>, card: RaiseCard) -> Result<CardRaised, Refusal> {
        self.operator_called(caller)?;
        let (title, read) = (card.title.trim(), card.read.trim());
        if title.is_empty() || read.is_empty() {
            return Err(self.card_refused("a card has a title and a read, and one of them is empty"));
        }
        self.card_options_checked(&card.options)?;
        let manifest_id = match card.repository.as_deref().map(str::trim).filter(|name| !name.is_empty()) {
            Some(name) => self.repository_id_of(name).ok_or_else(|| self.card_refused(&format!("{name} is no repository Fleet serves")))?,
            None => String::new(),
        };
        if let Some(talk_of) = &card.talk_of {
            self.card_known(talk_of).await?;
        }
        for session in &card.sessions {
            if self.store().lock().await.session(session.as_str()).map_err(|why| self.ledger_fault(why))?.is_none() {
                return Err(self.card_refused(&format!("{} is no Job Fleet knows", session.as_str())));
            }
        }
        let at = self.now();
        let id = format!("{CARD}{}", crate::session_host::new_id());
        let item = WaitingItem {
            order: 0,
            why: String::new(),
            id: id.clone(),
            text: title.to_string(),
            since: Instant::carried(at.as_str()),
            source: WaitingSource::Agent,
            act: Some(WaitingAct { kind: WaitingActKind::Answer, target: id.clone(), on: None }),
            options: options_of(&card.options),
            approves: false,
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
        let shown = OperatorCard { kind: card.kind, talk_of: card.talk_of.clone(), sessions: card.sessions.clone(), picked: None, steps: Vec::new(), returned: None };
        let row = KeptTurn {
            item: id,
            manifest_id,
            body: item_kept(&item),
            read_text: Some(read.to_string()),
            pick: card.options.first().map(|one| one.label.trim().to_string()),
            raised_at: at.as_str().to_string(),
            card: ipc::encode(&shown).ok(),
            ..KeptTurn::default()
        };
        let n = self.store().lock().await.add_operator_turn(&row).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.card_refused("that card stands already"))?;
        self.operator_page_changed();
        Ok(CardRaised { card: format!("turn:{n}") })
    }

    /// A card that stands, open or settled, named as a card `talk_of` carries on from.
    async fn card_known(&self, id: &str) -> Result<(), Refusal> {
        let n = turn_number(id).ok_or_else(|| self.card_refused(&format!("{id} is no card's id. A card's id is turn: and a number")))?;
        let row = self.store().lock().await.operator_turn_item(n).map_err(|why| self.ledger_fault(why))?;
        match row {
            Some(row) if row.card.is_some() => Ok(()),
            _ => Err(self.card_refused(&format!("{id} is no card Fleet holds"))),
        }
    }

    /// `revise_card`.
    pub(crate) async fn card_revised(self: &Arc<Self>, caller: Option<api::Caller>, revise: ReviseCard) -> Result<CardRevised, Refusal> {
        self.operator_called(caller)?;
        let row = self.card_open(&revise.card).await?;
        let mut item = ipc::operator_page::item_read(&row.body).ok_or_else(|| self.ledger_unnamed("a card that reads"))?;
        let read = match revise.read.as_deref().map(str::trim) {
            Some("") => return Err(self.card_refused("the read is empty. Leave it out to keep it")),
            Some(read) => Some(read.to_string()),
            None => row.read_text.clone(),
        };
        let mut pick = row.pick.clone();
        if let Some(options) = &revise.options {
            self.card_options_checked(options)?;
            item.options = options_of(options);
            pick = options.first().map(|one| one.label.trim().to_string());
        }
        let kept = self.store().lock().await.revise_operator_turn(row.n, &item_kept(&item), read.as_deref(), pick.as_deref()).map_err(|why| self.ledger_fault(why))?;
        if !kept {
            return Err(self.card_open(&revise.card).await.err().unwrap_or_else(|| self.card_refused("the card was settled first")));
        }
        self.operator_page_changed();
        Ok(CardRevised { card: revise.card })
    }

    /// `settle_card`.
    pub(crate) async fn card_settled(self: &Arc<Self>, caller: Option<api::Caller>, settle: SettleCard) -> Result<OperatorCardSettled, Refusal> {
        self.operator_called(caller)?;
        let said = settle.said.trim();
        if said.is_empty() {
            return Err(self.card_refused("`said` is empty. Say in a sentence how the card was settled"));
        }
        let row = self.card_open(&settle.card).await?;
        let at = self.now();
        self.store().lock().await.settle_operator_turn(row.n, at.as_str(), Some(&format!("{SETTLED}{said}"))).map_err(|why| self.ledger_fault(why))?;
        self.feed_kept(OperatorFeedKind::Settled, &Self::card_title(&row), None, Some(said)).await;
        Ok(OperatorCardSettled { card: settle.card })
    }

    /// The title of a card's row, as the Operator is told it.
    fn card_title(row: &KeptTurn) -> String {
        ipc::operator_page::item_read(&row.body).map(|item| item.text).unwrap_or_default()
    }

    /// His press of `label` on card `row`: picked, so it leaves the front for `working`, and the Operator told
    /// which option and to act now. **Refused with nothing changed** where the label is no option of it, the card
    /// is picked already and has not come back, or the Operator's session is not running.
    pub(crate) async fn card_pressed(self: &Arc<Self>, row: &KeptTurn, label: Option<&str>) -> Result<(), Refusal> {
        let item = ipc::operator_page::item_read(&row.body).ok_or_else(|| self.ledger_unnamed("a card that reads"))?;
        let label = label.or(row.pick.as_deref()).unwrap_or_default();
        if !item.options.iter().any(|option| option.label == label) {
            return Err(self.hosted_refusal("fleet.operator_answer_not_offered", &format!("{label} is not an option this card offers")));
        }
        let Some(session) = self.operator_session_id().await else {
            return Err(self.hosted_refusal(super::session::NOT_RUNNING, "the Operator's session is not running, so nothing here would reach it. Turn the Operator off and on to start it"));
        };
        let at = self.now();
        if !self.store().lock().await.pick_operator_turn(row.n, label, at.as_str()).map_err(|why| self.ledger_fault(why))? {
            return Err(self.hosted_refusal("fleet.operator_answer_not_offered", "this card is picked already and the Operator is working on it"));
        }
        self.operator_page_changed();
        let text = format!(
            "He chose \"{label}\" on card turn:{} \"{}\". Do it now. Report each real step with report_card, settle the card when it is done, \
             and report failed with the reason the moment it cannot be done. It comes back to his front if nothing settles it in 15 min.",
            row.n,
            Self::card_title(row)
        );
        let _ = self.told_of_his_act(&session, &text).await;
        Ok(())
    }

    /// `report_card`: a step on a picked card, or its failure, which puts it back first in Your turn.
    pub(crate) async fn card_reported(self: &Arc<Self>, caller: Option<api::Caller>, report: ReportCard) -> Result<CardRevised, Refusal> {
        self.operator_called(caller)?;
        let step = report.step.as_deref().map(str::trim);
        let failed = report.failed.as_deref().map(str::trim);
        let row = self.card_open(&report.card).await?;
        let at = self.now();
        let kept = {
            let mut store = self.store().lock().await;
            match (step, failed) {
                (Some(said), None) if !said.is_empty() => store.step_operator_turn(row.n, at.as_str(), said),
                (None, Some(why)) if !why.is_empty() => store.return_operator_turn(row.n, why, at.as_str()),
                _ => return Err(self.card_refused("give exactly one of `step` and `failed`, and say it in a sentence")),
            }
            .map_err(|why| self.ledger_fault(why))?
        };
        if !kept {
            return Err(self.card_refused(&format!("{} is not a card he picked that is being worked on", report.card)));
        }
        self.operator_page_changed();
        Ok(CardRevised { card: report.card })
    }

    /// Once a tick: each picked card nothing settled within [`STALL_MILLIS`] of the pick comes back to the front,
    /// and the Operator's session is told which.
    pub(crate) async fn cards_stalled_returned(self: &Arc<Self>) {
        let now = self.now();
        let by = crate::clock::rfc3339_utc(now.epoch_millis().unwrap_or(0) - STALL_MILLIS);
        let stalled = self.store().lock().await.operator_turn_picked_by(&by).unwrap_or_default();
        for row in stalled {
            match self.store().lock().await.return_operator_turn(row.n, STALLED, now.as_str()) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(why) => {
                    self.operator_warned("a picked card that stalled was not returned", why);
                    continue;
                }
            }
            self.operator_page_changed();
            let text = format!(
                "Card turn:{} \"{}\" is back in front of him: nothing settled it in 15 min after he chose \"{}\". Settle it if it is done; otherwise he picks again.",
                row.n,
                Self::card_title(&row),
                row.picked.as_deref().unwrap_or_default()
            );
            let _ = self.operator_told(&text).await;
        }
    }

    /// Card `row` sent back with Later until `until`: the Operator is told when it comes back.
    pub(crate) async fn card_sent_later(self: &Arc<Self>, row: &KeptTurn, until: &str) {
        let Some(session) = self.operator_session_id().await else { return };
        let text = format!("He sent card turn:{} \"{}\" back with Later. It comes back at {until}.", row.n, Self::card_title(row));
        let _ = self.told_of_his_act(&session, &text).await;
    }

    /// `talk_to_operator`: his words, delivered to the Operator's session as he wrote them. **With the switch on and no
    /// session running, they are kept**, a session is started at once, and it is told them once it is up. Refused with
    /// `fleet.operator_session_not_running` only with the switch off. A card he talks about is named to the session.
    /// The files and tags he sent go with the words, card or none. The session's answer is its thread, not this reply.
    pub(crate) async fn operator_talked(self: &Arc<Self>, talk: TalkToOperator) -> Result<OperatorSaid, Refusal> {
        if self.operator_session_id().await.is_some() {
            self.talk_delivered(talk).await?;
        } else if self.operator_on() {
            let kept = ipc::encode(&talk).map_err(|why| self.hosted_refusal(super::session::NOT_RUNNING, &format!("the words could not be kept: {why}")))?;
            self.store().lock().await.add_operator_held_send(&kept, self.now().as_str()).map_err(|why| self.ledger_fault(why))?;
            self.operator_seat().wake.notify_one();
        } else {
            return Err(self.hosted_refusal(super::session::NOT_RUNNING, "the Operator is off, so nothing here would hear it"));
        }
        Ok(OperatorSaid { said: String::new(), cites: Vec::new(), job: None, started: None, order: None, forms: Vec::new(), moved: None, forgot: None, lifted: None })
    }

    /// His words told to the running session: about the card he named, where Fleet holds it, else as he wrote them.
    pub(crate) async fn talk_delivered(self: &Arc<Self>, talk: TalkToOperator) -> Result<(), Refusal> {
        let TalkToOperator { words, about, attachments, mentions } = talk;
        let card = match about.as_deref() {
            Some(id) => self.card_known(id).await.is_ok().then_some(id),
            None => None,
        };
        match card {
            Some(card) => self.card_talked_about(card, &words, attachments, mentions).await,
            None => self.operator_told_as_him(&words, attachments, mentions).await,
        }
    }

    /// `operator_talk`: the conversation kept before the session, read as it was kept.
    pub(crate) async fn operator_talk_wire(&self) -> Result<OperatorTalk, Refusal> {
        let kept = self.store().lock().await.operator_talk(TALK_SHOWN).map_err(|why| self.ledger_fault(why))?;
        Ok(OperatorTalk { lines: kept.into_iter().map(|one| OperatorTalkLine { from_person: one.from_person, text: one.text, at: one.at, about: one.about }).collect() })
    }

    /// His words about a card, as he wrote them, with a short tag naming it. Refused where the card is not
    /// one Fleet holds or the Operator's session is not running.
    pub(crate) async fn card_talked_about(self: &Arc<Self>, id: &str, words: &str, attachments: Vec<SessionUpload>, mentions: Vec<SessionTag>) -> Result<(), Refusal> {
        self.card_known(id).await?;
        let n = turn_number(id).unwrap_or_default();
        let row = self.store().lock().await.operator_turn_item(n).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.card_refused(&format!("{id} is no card Fleet holds")))?;
        let tag = format!("[About card turn:{n} \"{}\"]", Self::card_title(&row));
        self.operator_told_as_him(&format!("{tag}\n{words}"), attachments, mentions).await
    }
}
