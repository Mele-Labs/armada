//! An open AskUserQuestion card as his items. **Each question is its own item** (the owner, 19 Nov 2026: one ask, one
//! item, one place): `ask:<call>` for the first and `ask:<call>#<n>` after it, each titled with its question and
//! carrying its own choices. His pick on one is held on the host until every question has one, and then the call is
//! answered with them all, in the order asked. `docs/concepts/session/waiting-on-the-person.md`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{CallInFlight, QuestionAnswer, WaitingAct, WaitingActKind, WaitingItem, WaitingOption, WaitingSource};

use crate::daemon::Fleet;

/// The item id of question `at` of the card `call`: the first keeps `ask:<call>`, and each after it is `#<at>`.
pub(crate) fn ask_item_id(call: &str, at: usize) -> String {
    match at {
        0 => format!("ask:{call}"),
        at => format!("ask:{call}#{at}"),
    }
}

/// Which question of its card an `ask:` item is, read back off its id.
pub(crate) fn ask_item_at(id: &str) -> usize {
    id.rsplit_once('#').and_then(|(_, at)| at.parse().ok()).unwrap_or(0)
}

/// The open card as items. **Each question is its own item** with its own choices, a pick of several among
/// them, so the sheet answers each like any other; one he has answered already is not listed. A card that asks
/// nothing is the one permission item.
pub(super) fn card_items(asked: &CallInFlight, picked: &[QuestionAnswer]) -> Vec<WaitingItem> {
    let act = Some(WaitingAct {
        kind: WaitingActKind::Answer,
        target: asked.call.clone(),
        on: None,
    });
    if !asked.questions.is_empty() {
        return asked
            .questions
            .iter()
            .enumerate()
            .filter(|(_, one)| !picked.iter().any(|done| done.question == one.question))
            .map(|(at, one)| WaitingItem {
                approves: false,
                id: ask_item_id(&asked.call, at),
                text: one.question.clone(),
                since: asked.asked_at.clone(),
                source: WaitingSource::AskCard,
                act: act.clone(),
                options: one
                    .options
                    .iter()
                    .map(|choice| WaitingOption {
                        label: choice.label.clone(),
                        description: Some(choice.description.clone()).filter(|said| !said.trim().is_empty()),
                        picture: None,
                    })
                    .collect(),
                steps: Vec::new(),
                place: None,
                confirmed: false,
                holders: Vec::new(),
                creating_since: None,
                create_refused: None,
                operator_read: None,
                artifact: None,
            })
            .collect();
    }
    vec![WaitingItem {
        approves: false,
        id: format!("perm:{}", asked.call),
        text: format!("{} {}", asked.tool, asked.detail)
            .trim()
            .to_string(),
        since: asked.asked_at.clone(),
        source: WaitingSource::Permission,
        act,
        options: asked
            .offers
            .iter()
            .map(|one| WaitingOption {
                label: super::waiting::offer_label(one).to_string(),
                description: None,
                picture: None,
            })
            .collect(),
        steps: Vec::new(),
        place: None,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
        artifact: None,
    }]
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
    /// The Job's open card, and his picks so far on it. Picks held for a card since answered are none.
    pub(super) fn card_and_picks(&self, id: &str) -> (Option<CallInFlight>, Vec<QuestionAnswer>) {
        let host = self.hosts().of(id);
        let state = host.state();
        let picked = state.ask_picks.as_ref().filter(|(call, _)| state.asked.as_ref().is_some_and(|card| card.call == *call)).map(|(_, picks)| picks.clone());
        (state.asked.clone(), picked.unwrap_or_default())
    }

    /// His answer to question `at` of the card `asked`, held: his pick, with his words as an entry of their own.
    /// **Every answer, in the order asked, once each question has one**, and `None` while one is still open.
    pub(super) fn card_picked(&self, id: &str, asked: &CallInFlight, at: usize, picked: Option<&str>, words: Option<&str>) -> Option<Vec<QuestionAnswer>> {
        let question = asked.questions.get(at)?;
        let chosen = match (picked, words) {
            (Some(pick), Some(words)) => vec![pick.to_string(), words.to_string()],
            (Some(one), None) | (None, Some(one)) => vec![one.to_string()],
            (None, None) => return None,
        };
        let host = self.hosts().of(id);
        let mut state = host.state();
        let mut picks = state.ask_picks.take().filter(|(call, _)| *call == asked.call).map(|(_, picks)| picks).unwrap_or_default();
        picks.retain(|one| one.question != question.question);
        picks.push(QuestionAnswer { question: question.question.clone(), chosen });
        let whole = asked.questions.iter().all(|one| picks.iter().any(|done| done.question == one.question));
        if !whole {
            state.ask_picks = Some((asked.call.clone(), picks));
            return None;
        }
        Some(asked.questions.iter().filter_map(|one| picks.iter().find(|done| done.question == one.question).cloned()).collect())
    }
}
