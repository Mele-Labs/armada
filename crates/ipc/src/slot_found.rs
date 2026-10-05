//! What a rescue Scout answers with, and how it is read back. The brief is
//! `docs/contracts/agent-prompt.md`, section 5d.
//!
//! **Decoded on the seam that decodes**, as a read-in's answer is, because it is
//! text a model wrote. An answer that is not the shape is kept by Fleet as the
//! Scout's own words rather than refused: the person is deciding on it.

use serde::{Deserialize, Serialize};

use crate::holding::SlotVerdict;
use crate::read_in::fenced;

/// The most items an `unfinished` verdict lists.
pub const MOST_ITEMS: usize = 8;
/// The most characters one item holds. An item is a line, not a paragraph.
pub const MOST_ITEM_CHARACTERS: usize = 200;

/// A verdict and its items, as the Scout gave them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotFound {
    pub verdict: SlotVerdict,
    #[serde(default)]
    pub items: Vec<String>,
}

impl SlotFound {
    /// Blank items dropped and the rest cut to the bounds; under `scraps`, only
    /// the first line, since that verdict says what the leftovers are in one.
    fn bounded(mut self) -> SlotFound {
        let cut = |item: &str| {
            item.trim()
                .chars()
                .take(MOST_ITEM_CHARACTERS)
                .collect::<String>()
        };
        self.items = self
            .items
            .iter()
            .map(|item| cut(item))
            .filter(|item| !item.is_empty())
            .take(match self.verdict {
                SlotVerdict::Unfinished => MOST_ITEMS,
                SlotVerdict::Scraps => 1,
            })
            .collect();
        self
    }
}

/// What a rescue Scout concluded, read out of the answer it ended its turn
/// with. **The last fenced block, not the first**, as `what_a_scout_read_in`.
pub fn what_a_scout_found_in_a_slot(answered: &str) -> Result<SlotFound, crate::Undecodable> {
    let block = fenced(answered).unwrap_or(answered.trim());
    crate::decode::<SlotFound>("what a scout found in a slot", block.as_bytes())
        .map(SlotFound::bounded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_fenced_block_is_the_verdict() {
        let answered = "Reading it.\n\n```json\n{\"verdict\":\"unfinished\",\
                        \"items\":[\"Call the new loop from src/lib.rs\",\"Write its test\"]}\n```\n";
        let found = what_a_scout_found_in_a_slot(answered).expect("the block decodes");
        assert_eq!(found.verdict, SlotVerdict::Unfinished);
        assert_eq!(
            found.items,
            ["Call the new loop from src/lib.rs", "Write its test"]
        );
    }

    /// Prose, or a verdict that is neither word, is not a verdict.
    #[test]
    fn an_answer_that_is_not_the_shape_is_refused() {
        assert!(what_a_scout_found_in_a_slot("The parser is half written.").is_err());
        assert!(what_a_scout_found_in_a_slot(
            "```json\n{\"verdict\":\"mostly\",\"items\":[]}\n```"
        )
        .is_err());
    }

    /// **Scraps is one line.** A second item is dropped, blanks go, and an
    /// unfinished list is cut to the bounds.
    #[test]
    fn items_are_bounded_and_scraps_keeps_one_line() {
        let scraps = "{\"verdict\":\"scraps\",\"items\":[\" \",\"A draft note\",\"A second\"]}";
        let found = what_a_scout_found_in_a_slot(scraps).expect("decodes");
        assert_eq!(found.items, ["A draft note"]);

        let long: Vec<String> = (0..MOST_ITEMS + 3)
            .map(|n| format!("\"{n}{}\"", "x".repeat(300)))
            .collect();
        let unfinished = format!(
            "{{\"verdict\":\"unfinished\",\"items\":[{}]}}",
            long.join(",")
        );
        let found = what_a_scout_found_in_a_slot(&unfinished).expect("decodes");
        assert_eq!(found.items.len(), MOST_ITEMS);
        assert!(found
            .items
            .iter()
            .all(|item| item.chars().count() == MOST_ITEM_CHARACTERS));
    }
}
