//! What the fake forge does with a served checkout's uncommitted files sent up
//! as a pull request. Its own file for `manifest_edits`' reason: it is scripted
//! through `&self`, because nobody presses the button until the Fleet holding
//! this fake exists.

use adapter_traits::{CheckoutSent, CheckoutToSend, NotDelivered};

use super::FakeVcs;

/// What the fake answers about sends, and every one it was given.
#[derive(Debug, Default)]
pub(super) struct CheckoutSends {
    /// What the push or the pull request refuses with. Absent sends cleanly.
    pub(super) refusing: Option<String>,
    pub(super) sent: Vec<CheckoutToSend>,
    /// Held shut by `hold_checkout_sends`.
    pub(super) held: Option<std::sync::Arc<super::Hold>>,
}

impl FakeVcs {
    /// Say that sending the checkout's files would not go: the words are the tool's.
    pub fn now_sending_the_checkout_is_refused(&self, said: &str) {
        self.checkout_sends.lock().expect("not poisoned").refusing = Some(said.to_string());
    }

    /// Every send of the served checkout's files, in order.
    pub fn checkouts_sent(&self) -> Vec<CheckoutToSend> {
        self.checkout_sends.lock().expect("not poisoned").sent.clone()
    }

    /// Recorded, then answered: a pull request at `.../pull/2402` unless refused.
    pub(super) fn checkout_sent(&self, sending: &CheckoutToSend) -> Result<CheckoutSent, NotDelivered> {
        let held = self.checkout_sends.lock().expect("not poisoned").held.clone();
        if let Some(held) = held {
            held.pass();
        }
        let number = {
            let mut sends = self.checkout_sends.lock().expect("not poisoned");
            sends.sent.push(sending.clone());
            if let Some(said) = &sends.refusing {
                return Err(NotDelivered::of("the push", said.clone()));
            }
            sends.sent.len()
        };
        let url = format!("https://forge.invalid/armada/pull/{}", 2401 + number);
        let auto_merge = self.auto_merge_asked(&url);
        Ok(CheckoutSent {
            commit: format!("{number:040x}"),
            pull_request: url,
            auto_merge,
        })
    }
}
