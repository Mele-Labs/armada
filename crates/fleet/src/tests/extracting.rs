//! Every ask of a reply's close as its own item, with the act its words name, and the follow-ups the
//! agent keeps. Every reply here is written for the test; `docs/spikes/reply-sentence-readers.md` has
//! the two heads measured on real ones, and the probabilities here stand in for them.

use std::sync::Arc;
use std::time::{Duration, Instant};

use adapter_traits::{NotEmbedded, TextModel, TextReading};
use ipc::{WaitingActKind, WaitingSource};

use crate::session_host::asks::closing;
use crate::session_host::asks_model::Reads;
use crate::session_host::extracting::{extracted, options_of, read, Act, Extracted, ASKS_AT, FOLLOWS_UP_AT};
use crate::session_host::standing::{derived, Seen};

const ENDED: &str = "2026-10-21T10:00:00Z";

fn sentences(reply: &str) -> Vec<String> {
    closing(reply).into_iter().map(|one| one.sentence).collect()
}

/// A heading introduces what follows and asks nothing: a `#` line, a bold-only line, a short line
/// ending in `:` or `?` alone before the next paragraph. A line a command follows is kept.
#[test]
fn headings_are_left_out_of_the_close() {
    assert_eq!(sentences("Both fixes are pushed.\n\n## Bigger than it looked?\n\nThe parser grew. Want me to split it?"), ["The parser grew.", "Want me to split it?"]);
    assert_eq!(sentences("Both fixes are pushed.\n\n**Your call**\n- Drop the flag.\n- Keep it behind a setting.\n\nNothing else changed."), ["Both fixes are pushed.", "Drop the flag.", "Keep it behind a setting.", "Nothing else changed."]);
    assert_eq!(sentences("Done.\n\nChoices for the label:\n\na) Rename the tab\nb) Leave it\n\nWhich do you want?"), ["Rename the tab", "Leave it", "Which do you want?"]);
    let handed = closing("The branch is still there.\n\nTo clear it yourself:\n\n```bash\ngit branch -D old-work\n```");
    assert_eq!(handed.last().map(|one| (one.sentence.as_str(), one.command.as_deref())), Some(("To clear it yourself:", Some("git branch -D old-work"))));
}

/// Scores as a head would give them: 1 for a sentence `yes` names, 0 otherwise.
fn scores(reply: &str, yes: &[&str]) -> Vec<f32> {
    closing(reply).iter().map(|one| if yes.contains(&one.sentence.as_str()) { 0.99 } else { 0.01 }).collect()
}

fn asks_of(reply: &str, asks: &[&str], later: &[&str]) -> Extracted {
    extracted(reply, &closing(reply), &scores(reply, asks), &scores(reply, later), None)
}

/// The act is read from the sentence: a fenced command is run, a pull request named with approve, merge
/// or land is approved, anything else is answered.
#[test]
fn each_ask_carries_the_act_its_words_name() {
    let run = asks_of("Cleanup is blocked from here. Run this in your checkout:\n\n```bash\nrm scratch.txt\n```", &["Run this in your checkout:"], &[]);
    assert_eq!(run.asks.len(), 1);
    assert_eq!(run.asks[0].act, Act::Run(String::from("rm scratch.txt")));
    let approve = asks_of("Tests pass.\n\nThe change is on its branch. Approve #12 when you are happy with it.", &["Approve #12 when you are happy with it."], &[]);
    assert_eq!(approve.asks[0].act, Act::ApprovePr(String::from("#12")));
    let merge = asks_of("Tests pass.\n\nIt lands once you merge #340.", &["It lands once you merge #340."], &[]);
    assert_eq!(merge.asks[0].act, Act::ApprovePr(String::from("#340")));
    let named = asks_of("Tests pass.\n\nThe fix for #12 is in. Do you want the docs touched too?", &["Do you want the docs touched too?"], &[]);
    assert_eq!(named.asks[0].act, Act::Answer, "a number named without approve, merge or land is not a pull request to approve");
}

/// The rules' sentence stays the first ask, so nothing the asks reader catches today is lost, and a
/// sentence read as both is an ask, not a follow-up.
#[test]
fn the_rules_sentence_is_the_first_ask_and_the_rest_follow_in_order() {
    let reply = "The cap is in.\n\nApprove #12 when you are happy with it. Which page size do you want? I'll trim the logging after it lands.";
    let got = asks_of(reply, &["Approve #12 when you are happy with it.", "Which page size do you want?"], &["I'll trim the logging after it lands.", "Which page size do you want?"]);
    let asked: Vec<(usize, &str)> = got.asks.iter().map(|ask| (ask.index, ask.sentence.as_str())).collect();
    assert_eq!(asked, [(0, "Which page size do you want?"), (1, "Approve #12 when you are happy with it.")]);
    assert_eq!(got.later, ["I'll trim the logging after it lands."]);
    let none = asks_of("The cap is in.\n\nThe tests pass.", &[], &[]);
    assert_eq!(none, Extracted::default());
}

#[test]
fn the_thresholds_are_probabilities() {
    for at in [ASKS_AT, FOLLOWS_UP_AT] {
        assert!(at > 0.0 && at < 1.0, "{at}");
    }
}

fn options(reply: &str, sentence: &str) -> Vec<String> {
    let close = closing(reply);
    options_of(sentence, &close, close.iter().position(|one| one.sentence == sentence))
}

/// "X, or Y?" offers X and Y, each without the question's lead; a question closing a list offers its
/// lines; a plain question offers nothing, and Bridge draws the reply box.
#[test]
fn a_question_that_names_its_choices_offers_them() {
    let either = "Should the registry be read once at start, or on every Studio open?";
    assert_eq!(options(either, either), ["Once at start", "On every Studio open"]);
    let land = "Should I land it or wait for ci?";
    assert_eq!(options(land, land), ["Land it", "Wait for ci"]);
    let three = "Do you want it cached in memory, on disk, or not at all?";
    assert_eq!(options(three, three), ["Cached in memory", "On disk", "Not at all"]);
    let stem = "Which page size do you want, 20 or 50?";
    assert_eq!(options(stem, stem), ["20", "50"]);
    let listed = "Two ways to go.\n\n- Keep the table and add a column.\n- Split it in two.\n\nLet me know which you prefer.";
    assert_eq!(options(listed, "Let me know which you prefer."), ["Keep the table and add a column", "Split it in two"]);
    let picked = "There are three options:\n\n1. Cache it in memory.\n2. Cache it on disk.\n3. Skip the cache.\n\nPick one of these.";
    assert_eq!(options(picked, "Pick one of these."), ["Cache it in memory", "Cache it on disk", "Skip the cache"]);
    let plain = "Which one do you want fixed first?";
    assert!(options(plain, plain).is_empty());
    let item = "Both fixes are pushed.\n\n- Walk the new page.\n- Say whether the copy reads right.";
    assert!(options(item, "Say whether the copy reads right.").is_empty(), "a list item closes no list");
}

/// Reads a sentence that ends in `?` or starts with Approve as an ask, and one that says what the agent
/// will do as a follow-up.
struct Fake;

impl TextModel for Fake {
    fn model(&self) -> &str {
        "fake@0"
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded> {
        Ok(texts.iter().map(|_| vec![0.0; 4]).collect())
    }

    fn read(&self, reading: TextReading, texts: &[&str]) -> Result<Vec<f32>, NotEmbedded> {
        let yes = |text: &str| match reading {
            TextReading::SentenceAsks => text.ends_with('?') || text.starts_with("Approve"),
            TextReading::SentenceFollowsUp => text.starts_with("I'll"),
            _ => panic!("{reading:?} is not a sentence reading"),
        };
        Ok(texts.iter().map(|text| if yes(text) { 0.99 } else { 0.01 }).collect())
    }
}

const TWO_ASKS: &str = "The cap is in.\n\nApprove #12 when you are happy with it. Should the page be read once at start, or on every open? I'll trim the logging after it lands.";

fn between<'a>(extracted: Option<&'a dyn Fn(&str) -> Option<Extracted>>) -> Seen<'a> {
    Seen {
        helm: false,
        running: None,
        subagents: false,
        held: false,
        called: false,
        turn_end: Some((ENDED, TWO_ASKS)),
        spoke: None,
        done: None,
        dismissed: &[],
        asks: &crate::session_host::asks::asking_line,
        extracted,
        tell: None,
    }
}

/// With the readers on, a reply that asks twice is two items, each with its act, numbered by ask; the
/// follow-up is kept apart. With them off, the one item as before.
#[tokio::test]
async fn with_the_readers_on_each_ask_is_an_item_and_off_the_reply_is_one() {
    let reads: Arc<Reads<Extracted>> = Arc::new(Reads::default());
    let model: Arc<dyn TextModel> = Arc::new(Fake);
    let by_model = |said: &str| reads.read_of(read, Arc::clone(&model), "s1", said);
    // The first derive starts the read and answers by the rules, as the first ask.
    let pending = derived(&between(Some(&by_model)));
    assert_eq!(pending.iter().map(|one| (one.id.as_str(), one.text.as_str())).collect::<Vec<_>>(), [(&*format!("replied:{ENDED}:0"), "Should the page be read once at start, or on every open?")]);
    let deadline = Instant::now() + Duration::from_secs(30);
    while reads.settled().is_empty() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let got = by_model(TWO_ASKS).expect("the fake has read it");
    assert_eq!(got.later, ["I'll trim the logging after it lands."]);
    let items = derived(&between(Some(&by_model)));
    let seen: Vec<(String, WaitingSource, String, Option<WaitingActKind>, Vec<String>)> = items
        .into_iter()
        .map(|one| (one.id, one.source, one.text, one.act.as_ref().map(|act| act.kind), one.options.into_iter().map(|option| option.label).collect()))
        .collect();
    assert_eq!(
        seen,
        [
            (format!("replied:{ENDED}:0"), WaitingSource::Replied, String::from("Should the page be read once at start, or on every open?"), Some(WaitingActKind::Answer), vec![String::from("Once at start"), String::from("On every open")]),
            (format!("replied:{ENDED}:1"), WaitingSource::Replied, String::from("Approve #12 when you are happy with it."), Some(WaitingActKind::ApprovePr), Vec::new()),
        ]
    );
    let off = derived(&between(None));
    assert_eq!(off.iter().map(|one| (one.id.as_str(), one.act.is_none())).collect::<Vec<_>>(), [(&*format!("replied:{ENDED}"), true)]);
}

/// An ask he dismissed stays dismissed by its own number, and the others still show.
#[test]
fn a_dismissed_ask_leaves_the_others_standing() {
    let all = |said: &str| Some(asks_of(said, &["Approve #12 when you are happy with it."], &[]));
    let gone = vec![format!("replied:{ENDED}:0")];
    let seen = Seen { dismissed: &gone, ..between(Some(&all)) };
    assert_eq!(derived(&seen).iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [format!("replied:{ENDED}:1")]);
}

/// As `asks_of`, with the clarity reader in force: `unclear` names the sentences the head finds
/// unclear (0.9 unclear, else 0.1), and the rules read the rest.
fn clear_asks_of(reply: &str, asks: &[&str], unclear: &[&str]) -> Extracted {
    let close = closing(reply);
    let clear: Vec<f32> = close.iter().map(|one| if unclear.contains(&one.sentence.as_str()) { 0.1 } else { 0.9 }).collect();
    extracted(reply, &close, &scores(reply, asks), &scores(reply, &[]), Some(&clear))
}

/// "Bigger than it looked?" was the rules' sentence, a heading shown as his question. Off, it is the
/// ask as before; in force, it is not an ask, and a reply with no other raises nothing.
#[test]
fn with_clarity_in_force_a_heading_the_rules_picked_is_not_an_ask() {
    let reply = "The parser grew past 900 lines.\n\nBigger than it looked?";
    assert_eq!(asks_of(reply, &[], &[]).asks.iter().map(|ask| ask.sentence.as_str()).collect::<Vec<_>>(), ["Bigger than it looked?"]);
    assert_eq!(clear_asks_of(reply, &[], &[]), Extracted::default());
}

/// The clear ask stays and takes the first number, so its id is the same however many unclear ones
/// came before it.
#[test]
fn the_clear_asks_stay_and_are_numbered_among_themselves() {
    let reply = "The parser grew past 900 lines.\n\nBigger than it looked? Should I split it into two files?";
    let asked = &["Bigger than it looked?", "Should I split it into two files?"];
    assert_eq!(asks_of(reply, asked, &[]).asks.len(), 2);
    let got = clear_asks_of(reply, asked, &[]);
    assert_eq!(got.asks.iter().map(|ask| (ask.index, ask.sentence.as_str())).collect::<Vec<_>>(), [(0, "Should I split it into two files?")]);
}

/// The head can find a sentence unclear that the rules pass.
#[test]
fn a_sentence_the_head_finds_unclear_is_left_out_though_the_rules_pass_it() {
    let reply = "The cap is in.\n\nShould I split it into two files? Should I land it?";
    let asked = &["Should I split it into two files?", "Should I land it?"];
    let got = clear_asks_of(reply, asked, &["Should I split it into two files?"]);
    assert_eq!(got.asks.iter().map(|ask| ask.sentence.as_str()).collect::<Vec<_>>(), ["Should I land it?"]);
}

/// Through `derived`: a reply whose only ask is unclear raises no item, and reads as a reply, not as
/// one that waits on him.
#[test]
fn a_reply_with_no_clear_ask_raises_no_item() {
    let reply = "The parser grew past 900 lines.\n\nBigger than it looked?";
    let forced = |said: &str| Some(clear_asks_of(said, &[], &[]));
    let seen = Seen { turn_end: Some((ENDED, reply)), ..between(Some(&forced)) };
    assert!(derived(&seen).is_empty());
    assert_eq!(crate::session_host::standing::standing(&seen, Duration::from_secs(600)), ipc::SessionStanding::Replied);
}
