//! Teaching, as the fake answers it: words for what it was handed, and each act echoed.

use std::sync::Arc;

use ipc::{ReadTeaching, SaveTeaching, SavedTeaching, TeachingProposal, TeachingWords, WordRules};

use super::FakeDaemon;
use crate::{Refusal, Teaching};

impl Teaching for FakeDaemon {
    async fn read_teaching(&self, teach: ReadTeaching) -> Result<TeachingProposal, Refusal> {
        let words = TeachingWords {
            name: "Key".to_string(),
            pattern: "[A-Z]+-\\d+".to_string(),
            example: teach.words,
            address: Some(ipc::ProposedAddress { template: "https://x.example/{match}".to_string(), found: ipc::AddressFound::Person }),
        };
        Ok(TeachingProposal { words: Some(words), ..TeachingProposal::default() })
    }

    async fn save_teaching(self: Arc<Self>, save: SaveTeaching) -> Result<SavedTeaching, Refusal> {
        Ok(SavedTeaching { entry: save.proposal.note.map(|_| "7".to_string()), mods: vec!["key".to_string()], rule: None })
    }

    async fn list_word_rules(self: Arc<Self>, _repository: Option<String>, _session_id: Option<String>) -> Result<WordRules, Refusal> {
        Ok(WordRules { rules: Vec::new() })
    }
}
