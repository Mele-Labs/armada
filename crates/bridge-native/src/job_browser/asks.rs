//! A page's ask: its words and its life (`shared/browser-ask.ts`, and the ask in `main/job-browsers.ts`).
//!
//! **One ask at a time a tab.** A page asks and waits; the owner answers in the tab's strip or in Needs you, and either settles both.
//! A second ask while one waits is refused as dismissed. A dismissal is remembered by nobody.

use ipc::WaitingOption;

use super::choices::{Answer, Choice, Permission};

/// What Local network reaches, said under its Allow and under the strip's words.
pub const LOCAL_NETWORK_RISK: &str = "It can reach apps on this Mac and devices on your network.";

/// The question Needs you carries: one permission, and the site that wants it.
pub fn text(permission: Permission, host: &str) -> String {
    format!("Allow {} for {host}?", permission.label())
}

/// Allow and Block, in that order: the index Fleet answers with is 0 for Allow and 1 for Block.
pub fn options(permission: Permission) -> Vec<WaitingOption> {
    let allow = WaitingOption { label: "Allow".to_string(), description: (permission == Permission::LocalNetwork).then(|| LOCAL_NETWORK_RISK.to_string()), picture: None };
    let block = WaitingOption { label: "Block".to_string(), description: None, picture: None };
    vec![allow, block]
}

/// What the strip says over the page.
pub fn strip_words(permission: Permission, host: &str) -> String {
    format!("{host} wants to use {}", permission.label())
}

/// What the strip says under its words, where the permission has a risk to name.
pub fn strip_risk(permission: Permission) -> Option<&'static str> {
    (permission == Permission::LocalNetwork).then_some(LOCAL_NETWORK_RISK)
}

/// What Fleet's `browser.ask_answered` says: 0 is Allow, 1 is Block, and null or anything else is dismissed.
pub fn answer_of(choice: Option<u32>) -> Answer {
    match choice {
        Some(0) => Answer::Allow,
        Some(1) => Answer::Block,
        _ => Answer::Dismiss,
    }
}

/// The ask a tab waits on: what is asked, of whom, and what answers the page that asked (`S`: a handler the page left).
pub struct Asking<S> {
    pub permission: Permission,
    pub host: String,
    pub settle: S,
}

/// How an ask ended, and what Fleet is to be told of it.
pub struct Ended<S> {
    pub asking: Asking<S>,
    /// Whether Fleet is told the ask is over: every end but Fleet's own answer.
    pub tell_fleet: bool,
}

/// A tab's one ask.
pub struct AskSlot<S> {
    asking: Option<Asking<S>>,
}

impl<S> Default for AskSlot<S> {
    fn default() -> Self {
        AskSlot { asking: None }
    }
}

impl<S> AskSlot<S> {
    /// An ask is taken where none waits. Else it is handed back to be answered as dismissed.
    pub fn raise(&mut self, permission: Permission, host: String, settle: S) -> Result<(), S> {
        if self.asking.is_some() {
            return Err(settle);
        }
        self.asking = Some(Asking { permission, host, settle });
        Ok(())
    }

    pub fn asking(&self) -> Option<&Asking<S>> {
        self.asking.as_ref()
    }

    /// The ask is over. `fleet_knows`: it was Fleet that said so, so there is nothing to end. `held`: the tab has an id Fleet knows it
    /// by; a page opened before there was a tab has none, and Fleet was never told of its ask (`UNATTACHED`).
    pub fn end(&mut self, fleet_knows: bool, held: bool) -> Option<Ended<S>> {
        self.asking.take().map(|asking| Ended { asking, tell_fleet: !fleet_knows && held })
    }
}

/// What deciding a request asks of whoever holds the choices and the ask (`Rules` in `capture/site-permissions.ts`).
pub trait Rules {
    /// The choice for a permission now, for the page's host.
    async fn choice(&self, permission: Permission) -> Choice;
    /// Asked of the owner: his answer, or a dismissal where the tab has an ask waiting already or is gone.
    async fn ask(&self, permission: Permission) -> Answer;
    /// An answer kept for the host.
    async fn remember(&self, permission: Permission, choice: Choice);
}

/// Whether a request is given: from the saved choices, and asked of the owner where the choice is Ask (`decide`). Every permission it
/// names must be given. Asked in order, so a Block, or a dismissal, ends the asking.
pub async fn decide<R: Rules>(wants: &[Permission], rules: &R) -> bool {
    for want in wants {
        match rules.choice(*want).await {
            Choice::Block => return false,
            Choice::Allow => continue,
            Choice::Ask => {
                let answer = rules.ask(*want).await;
                let Some(kept) = answer.remembered() else { return false };
                rules.remember(*want, kept).await;
                if answer == Answer::Block {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    /// Rules with fixed choices and one answer for every ask, writing down what was asked and kept.
    struct Fake {
        choices: [(Permission, Choice); 2],
        answer: Answer,
        asked: RefCell<Vec<Permission>>,
        kept: RefCell<Vec<(Permission, Choice)>>,
    }

    impl Fake {
        fn new(camera: Choice, microphone: Choice, answer: Answer) -> Fake {
            Fake { choices: [(Permission::Camera, camera), (Permission::Microphone, microphone)], answer, asked: RefCell::default(), kept: RefCell::default() }
        }
    }

    impl Rules for Fake {
        async fn choice(&self, permission: Permission) -> Choice {
            self.choices.iter().find(|(one, _)| *one == permission).map_or(Choice::Ask, |(_, choice)| *choice)
        }
        async fn ask(&self, permission: Permission) -> Answer {
            self.asked.borrow_mut().push(permission);
            self.answer
        }
        async fn remember(&self, permission: Permission, choice: Choice) {
            self.kept.borrow_mut().push((permission, choice));
        }
    }

    const BOTH: [Permission; 2] = [Permission::Camera, Permission::Microphone];

    #[test]
    fn allow_grants_and_block_denies_without_asking_and_ask_raises_the_ask() {
        let allowed = Fake::new(Choice::Allow, Choice::Allow, Answer::Dismiss);
        assert!(gpui::block_on(decide(&BOTH, &allowed)));
        assert!(allowed.asked.borrow().is_empty());
        let blocked = Fake::new(Choice::Allow, Choice::Block, Answer::Allow);
        assert!(!gpui::block_on(decide(&BOTH, &blocked)));
        assert!(blocked.asked.borrow().is_empty(), "a Block is refused where it stands");
        let asking = Fake::new(Choice::Ask, Choice::Allow, Answer::Allow);
        assert!(gpui::block_on(decide(&BOTH, &asking)));
        assert_eq!(*asking.asked.borrow(), [Permission::Camera]);
    }

    #[test]
    fn an_answer_is_remembered_and_a_dismissal_is_refused_and_kept_by_nobody() {
        let allowed = Fake::new(Choice::Ask, Choice::Ask, Answer::Allow);
        assert!(gpui::block_on(decide(&BOTH, &allowed)));
        assert_eq!(*allowed.asked.borrow(), BOTH, "each is asked in turn");
        assert_eq!(*allowed.kept.borrow(), [(Permission::Camera, Choice::Allow), (Permission::Microphone, Choice::Allow)]);
        let blocked = Fake::new(Choice::Ask, Choice::Ask, Answer::Block);
        assert!(!gpui::block_on(decide(&BOTH, &blocked)));
        assert_eq!(*blocked.kept.borrow(), [(Permission::Camera, Choice::Block)], "a Block is kept and ends the asking");
        let dismissed = Fake::new(Choice::Ask, Choice::Ask, Answer::Dismiss);
        assert!(!gpui::block_on(decide(&BOTH, &dismissed)));
        assert!(dismissed.kept.borrow().is_empty(), "a dismissal is not remembered");
        assert_eq!(dismissed.asked.borrow().len(), 1);
    }

    #[test]
    fn the_question_and_its_two_options_are_classics_words() {
        assert_eq!(text(Permission::Camera, "meet.example.com"), "Allow Camera for meet.example.com?");
        assert_eq!(text(Permission::LocalNetwork, "localhost:3000"), "Allow Local network for localhost:3000?");
        let camera = options(Permission::Camera);
        assert_eq!(camera.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Allow", "Block"]);
        assert!(camera.iter().all(|one| one.description.is_none()));
        let network = options(Permission::LocalNetwork);
        assert_eq!(network[0].description.as_deref(), Some("It can reach apps on this Mac and devices on your network."), "the risk rides on Allow");
        assert_eq!(network[1].description, None);
    }

    #[test]
    fn the_strip_says_who_wants_what_and_names_the_risk_only_for_the_local_network() {
        assert_eq!(strip_words(Permission::Microphone, "a.com"), "a.com wants to use Microphone");
        assert_eq!(strip_risk(Permission::LocalNetwork), Some("It can reach apps on this Mac and devices on your network."));
        assert_eq!(strip_risk(Permission::Camera), None);
    }

    #[test]
    fn fleets_choice_index_reads_as_allow_block_or_dismissed() {
        assert_eq!(answer_of(Some(0)), Answer::Allow);
        assert_eq!(answer_of(Some(1)), Answer::Block);
        assert_eq!(answer_of(None), Answer::Dismiss);
        assert_eq!(answer_of(Some(7)), Answer::Dismiss);
    }

    #[test]
    fn a_second_ask_while_one_waits_is_handed_back_to_be_dismissed() {
        let mut slot = AskSlot::default();
        assert!(slot.raise(Permission::Camera, "a.com".into(), 1).is_ok());
        assert_eq!(slot.raise(Permission::Microphone, "a.com".into(), 2).err(), Some(2));
        assert_eq!(slot.asking().map(|one| (one.permission, one.host.as_str(), one.settle)), Some((Permission::Camera, "a.com", 1)), "the first stays");
        slot.end(false, true);
        assert!(slot.raise(Permission::Microphone, "a.com".into(), 3).is_ok(), "the tab asks again once it ended");
    }

    #[test]
    fn every_end_but_fleets_own_answer_tells_fleet() {
        let mut slot = AskSlot::default();
        slot.raise(Permission::Camera, "a.com".into(), ()).unwrap();
        assert!(slot.end(false, true).expect("an ask").tell_fleet, "answered or dismissed in the tab, closed, or gone");
        slot.raise(Permission::Camera, "a.com".into(), ()).unwrap();
        assert!(!slot.end(true, true).expect("an ask").tell_fleet, "Fleet answered it, and took the item off first");
        slot.raise(Permission::Camera, "a.com".into(), ()).unwrap();
        assert!(!slot.end(false, false).expect("an ask").tell_fleet, "a page with no tab was never raised");
        assert!(slot.end(false, true).is_none(), "a late answer to no ask does nothing");
    }
}
