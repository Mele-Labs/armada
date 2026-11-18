//! What the fake forge answers when pull requests are read by number, and each read asked.

use adapter_traits::{ListedPull, NumberedPull};

use super::FakeVcs;

/// Silent until a test scripts which pull requests are open.
#[derive(Debug, Default)]
pub(super) struct NumberedPulls {
    open: Option<Vec<ListedPull>>,
    /// The pull requests of each read, in order.
    asks: Vec<Vec<NumberedPull>>,
}

impl NumberedPulls {
    /// The open ones of `pulls`: those scripted, matched by repository and number. Any other is
    /// closed, as the forge leaves it out.
    pub(super) fn asked(&mut self, pulls: &[NumberedPull]) -> Option<Vec<ListedPull>> {
        self.asks.push(pulls.to_vec());
        let open = self.open.as_ref()?;
        Some(
            open.iter()
                .filter(|row| {
                    pulls.iter().any(|it| it.number == row.number && it.repository.eq_ignore_ascii_case(&row.repository))
                })
                .cloned()
                .collect(),
        )
    }
}

impl FakeVcs {
    /// The forge holds `rows` open, and reads any of them by number. Any other pull request read by
    /// number is closed.
    pub fn now_open_by_number(&self, rows: Vec<ListedPull>) {
        self.numbered_pulls.lock().expect("not poisoned").open = Some(rows);
    }

    /// Every read by number asked of the forge, in order.
    pub fn numbered_reads_asked(&self) -> Vec<Vec<NumberedPull>> {
        self.numbered_pulls.lock().expect("not poisoned").asks.clone()
    }
}
