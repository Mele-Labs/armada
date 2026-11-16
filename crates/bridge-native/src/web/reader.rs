//! What the reader asks of a page it shows: its headings for the contents, the words he selected for a note, and a scroll to a
//! block or to words he marked. **The page is told nothing**: each ask is `reader.js` evaluated in the finder's content world
//! (`ask` on the web page), and each answer is a small plain value read and bounded here.

use serde::Deserialize;

const SOURCE: &str = include_str!("reader.js");

/// One heading of the page, in the order the page holds them.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Head {
    pub level: u8,
    pub text: String,
    /// The block it is, `b3`.
    pub block: String,
}

/// The words he selected and the block they start in. Empty words: nothing was selected, and the block is the one at the
/// top of the view.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Picked {
    #[serde(default)]
    pub quote: String,
    #[serde(default)]
    pub block: String,
}

#[derive(Deserialize)]
struct Words {
    #[serde(default)]
    heads: Vec<Head>,
}

fn ask(call: &str) -> String {
    format!("{SOURCE}\n{call}")
}

fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into())
}

/// The script that lists the page's headings, after hiding a leading `h1` that says `title`, which the head says already.
pub fn words(title: &str) -> String {
    ask(&format!("ArmadaReader.words({})", quoted(title)))
}

/// The script that reads his selection.
pub fn selection() -> String {
    ask("ArmadaReader.selection()")
}

/// The script that scrolls block `block` to the top of the view.
pub fn go(block: &str) -> String {
    ask(&format!("ArmadaReader.go({})", quoted(block)))
}

/// The script that selects the words he marked and scrolls to them.
pub fn show(quote: &str, block: &str) -> String {
    ask(&format!("ArmadaReader.show({}, {})", quoted(quote), quoted(block)))
}

/// The headings an answer lists. At most 200, each cut to 200 characters: a page is a Job's own and may hold anything.
pub fn heads_of(answer: Option<&str>) -> Vec<Head> {
    let Some(answer) = answer else { return Vec::new() };
    let Ok(words) = ipc::decode::<Words>("page headings", answer.as_bytes()) else { return Vec::new() };
    words
        .heads
        .into_iter()
        .filter(|head| (1..=6).contains(&head.level) && !head.text.is_empty())
        .take(200)
        .map(|head| Head { text: head.text.chars().take(200).collect(), ..head })
        .collect()
}

/// His selection as an answer reads it, the words cut to 2000 characters.
pub fn picked_of(answer: Option<&str>) -> Option<Picked> {
    let picked: Picked = ipc::decode("page selection", answer?.as_bytes()).ok()?;
    Some(Picked { quote: picked.quote.chars().take(2000).collect(), ..picked })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_headings_are_read_in_order_and_what_is_not_one_is_dropped() {
        let answer = r#"{"heads":[{"level":1,"text":"Stale branch design","block":"b0"},{"level":2,"text":"","block":"b2"},{"level":9,"text":"x","block":"b3"},{"level":2,"text":"Stale branches","block":"b4"}]}"#;
        let heads = heads_of(Some(answer));
        assert_eq!(heads.iter().map(|head| (head.level, head.text.as_str(), head.block.as_str())).collect::<Vec<_>>(), [(1, "Stale branch design", "b0"), (2, "Stale branches", "b4")]);
        assert!(heads_of(None).is_empty() && heads_of(Some("null")).is_empty() && heads_of(Some("not json")).is_empty());
    }

    #[test]
    fn a_selection_is_words_and_a_block_and_none_reads_as_none() {
        assert_eq!(picked_of(Some(r#"{"quote":"four branches","block":"b1"}"#)), Some(Picked { quote: "four branches".into(), block: "b1".into() }));
        assert_eq!(picked_of(Some(r#"{"quote":"","block":"b7"}"#)).map(|one| one.quote), Some(String::new()));
        assert_eq!(picked_of(Some("null")), None);
        assert_eq!(picked_of(None), None);
    }

    #[test]
    fn the_headings_call_carries_the_title_so_a_leading_h1_that_repeats_it_can_be_hidden() {
        assert!(words("Stale branch design").ends_with(r#"ArmadaReader.words("Stale branch design")"#));
        let source = include_str!("reader.js");
        assert!(source.contains("first.tagName === \"H1\"") && source.contains("toLowerCase()"), "the title is matched trimmed and case-insensitive");
    }

    #[test]
    fn a_call_carries_its_words_as_json_so_a_quote_cannot_end_the_script() {
        let script = show("say \"hi\"\n</script>", "b2");
        assert!(script.ends_with(r#"ArmadaReader.show("say \"hi\"\n</script>", "b2")"#), "{script}");
        assert!(go("b3").ends_with(r#"ArmadaReader.go("b3")"#));
    }
}
