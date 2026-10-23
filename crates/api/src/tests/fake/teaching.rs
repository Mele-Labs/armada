//! Teaching, as the fake answers it: a link for the words it was handed, and each act echoed.

use std::sync::Arc;

use ipc::{
    ForgottenTaughtLink, Instant, ReadTeaching, SaveTeaching, SavedTeaching, TaughtLink, TaughtLinks, TeachingLink,
    TeachingProposal,
};

use super::FakeDaemon;
use crate::{Refusal, Teaching};

fn kept(link: TeachingLink, repository: Option<String>) -> TaughtLink {
    TaughtLink {
        id: "1".to_string(),
        pattern: link.pattern,
        address: link.address,
        example: link.example,
        repository,
        made_at: Instant::carried("2000-01-01T00:00:00.000Z"),
    }
}

impl Teaching for FakeDaemon {
    async fn read_teaching(&self, teach: ReadTeaching) -> Result<TeachingProposal, Refusal> {
        let link = TeachingLink { pattern: "[A-Z]+-\\d+".to_string(), address: "https://x.example/{match}".to_string(), example: teach.words };
        Ok(TeachingProposal { link: Some(link), note: None })
    }

    async fn save_teaching(self: Arc<Self>, save: SaveTeaching) -> Result<SavedTeaching, Refusal> {
        Ok(SavedTeaching { link: save.link.map(|link| kept(link, save.repository)) })
    }

    async fn list_taught_links(&self, _repository: Option<String>) -> Result<TaughtLinks, Refusal> {
        Ok(TaughtLinks { links: Vec::new() })
    }

    async fn forget_taught_link(&self, link_id: String) -> Result<ForgottenTaughtLink, Refusal> {
        Ok(ForgottenTaughtLink { id: link_id })
    }
}
