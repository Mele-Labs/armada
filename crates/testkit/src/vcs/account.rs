//! What the fake forge lists of the account's repositories, and how often it was asked.

use adapter_traits::{AccountRepositories, ForgeRepository};

use super::FakeVcs;

/// An empty list until a test scripts one.
#[derive(Debug, Default)]
pub(super) struct AccountListing {
    answer: Option<Result<AccountRepositories, String>>,
    /// The `most` of each ask, in order.
    asks: Vec<usize>,
}

impl AccountListing {
    pub(super) fn asked(&mut self, most: usize) -> Result<AccountRepositories, String> {
        self.asks.push(most);
        self.answer.clone().unwrap_or_else(|| Ok(AccountRepositories::default()))
    }
}

impl FakeVcs {
    /// The forge lists `repositories` for the account, and says the list was cut where `cut`.
    pub fn account_lists(&self, repositories: Vec<ForgeRepository>, cut: bool) {
        self.account.lock().expect("not poisoned").answer = Some(Ok(AccountRepositories { repositories, cut }));
    }

    /// The forge refuses to list the account's repositories, in its own sentence.
    pub fn account_listing_refuses(&self, said: &str) {
        self.account.lock().expect("not poisoned").answer = Some(Err(said.to_string()));
    }

    /// The `most` of every listing asked of the forge, in order.
    pub fn account_listings_asked(&self) -> Vec<usize> {
        self.account.lock().expect("not poisoned").asks.clone()
    }
}
