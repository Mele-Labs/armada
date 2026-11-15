//! The Operator page's walk: one card in front with its options, Discuss with a tag in the box, the talk under the folded
//! card and a card the Operator raises in it, Next up and a kept idea brought from it, Snooze, the feed, a card arriving, a
//! refused and a lost answer, and the Operator off and switched on. Fleet is the mock in `operator/scenes.rs`; a step that names a scene is
//! Fleet speaking first. The key map is the last step's.

use super::{answer, step, Step, Verb};

const FRONT: &str = "Two Jobs are changing paging.rs";
const TAG: &str = "About Two Jobs are changing paging.rs";
const RAISED: &str = "Hold Pocket for 20 minutes or move its change";
const NEXT: &str = "Main is red at 4c1d2e9";
const THIRD: &str = "Dark mode for the settings page needs a palette";
const LAST: &str = "Rate limits per token have no default";
const IDEA: &str = "Checkout shows the total twice after a coupon";
const HOLD: &str = "Hold Pocket until Jobs screens pushes, Pocket waits about 20 min";
const BOTH: &str = "Let both run, A conflict to fix, about 10 min";
/// The card the walk's Fleet has the Operator working on, its option and its title as the feed's row names it.
const WORKING_TITLE: &str = "Retry the flaky login test or skip it";
const WORKING: &str = "Retry it once, Retry the flaky login test or skip it";
/// The row the press on the first option adds to the feed.
const JUST_PICKED: &str = "Hold Pocket until Jobs screens pushes, Two Jobs are changing paging.rs";
const ACTS: &str = "What the Operator does unasked, by project";
/// Fleet's why for a Snooze whose time has passed, as the scene `late` has it refuse.
const LATE: &str = "That time has passed. Pick a later one.";

pub static STEPS: [Step; 44] = [
    step("The head: Operator, the red main as a pill, and at the right what the Operator does unasked, by project, since the window is on All", Verb::Look, "button", ACTS),
    step("Its menu has each project as a header with its three values under it: Act then tell, Ask before starting or Only when told, ticked where set", Verb::Open("button", ACTS), "menuitemcheckbox", "Ask before starting"),
    step("A press sets that project's value at once", Verb::Open("menuitemcheckbox", "Ask before starting"), "button", ACTS),
    step("One card is in front: its kind, its title, a chip for each Job it names and its repository", Verb::Look, "article", FRONT),
    step("My read says what it found. The options are rows: the key at the left, the cost under each, and My pick on the first", Verb::Look, "button", HOLD),
    step("A hover lifts a row with an accent border and shows Choose at its right. The cost is in its tooltip too", Verb::Hover, "button", BOTH),
    step("Snooze puts the card back until a time, and Discuss talks to the Operator about it", Verb::Look, "button", "Discuss"),
    step("Discuss puts the keys in the box under the card, with a tag naming the card. The card stays in sight", Verb::Open("button", "Discuss"), "group", TAG),
    step("Typing is only typing: a, l, / and 2 press nothing", Verb::Typed("Why hold Pocket and not the other Job? Is a 2 hour wait worse?"), "textbox", "Message"),
    step("Enter sends it as a talk about the card, which folds to one line, with its chevron", Verb::Key("enter"), "button", "Unfold Two Jobs are changing paging.rs"),
    step("His words are the first of the talk, under the folded card", Verb::Look, "listitem", "You: Why hold Pocket and not the other Job? Is a 2 hour wait worse?"),
    answer("The Operator answers in its thread, and raises a card in the talk", "said", "listitem", "Operator: Pocket's change is small. Holding it for 20 minutes costs less than the conflict. The other way is to move it into Jobs screens, which takes one Job off the files."),
    step("The card it raised is whole in the talk, indented under its line. The box stays at the foot, with the keys in it", Verb::Look, "article", RAISED),
    step("Esc gives the keys to the page", Verb::Key("escape"), "article", RAISED),
    step("2 settles the card in play with its second option at once", Verb::Key("2"), "article", NEXT),
    step("Its talk is done and the next card is in front. Next up, at the top of the right column above the feed, is folded, with its count: three queued cards and three kept ideas", Verb::Look, "button", "Next up 6"),
    step("Next up opens, and the keys go into it: the queued cards first, each its kind, its title and its Job or repository", Verb::Open("button", "Next up 6"), "button", THIRD),
    step("↓ moves through the rows, and Enter brings that card to the front", Verb::Keys(&["down", "enter"]), "article", THIRD),
    step("The kept ideas follow the queued cards, newest first, each with its kind chip and how long ago it was kept", Verb::Look, "button", IDEA),
    step("A press on an idea brings it to the front as a card: its kind, its words, its repository and when it was kept, and two options. It has no Discuss and no Snooze", Verb::Open("button", IDEA), "article", IDEA),
    step("2 lets the idea go, and the Operator's own card is back in front", Verb::Key("2"), "article", THIRD),
    step("S opens Snooze's menu: when the card comes back, each choice with its time", Verb::Key("s"), "menuitem", "Tomorrow morning"),
    step("A time puts the card back, and the next one comes to the front", Verb::Open("menuitem", "Tomorrow morning"), "article", LAST),
    answer("Fleet will refuse the next Snooze, as the time it names has passed", "late", "button", "Snooze"),
    step("S opens the times again", Verb::Key("s"), "menuitem", "1 hour"),
    step("Enter on 1 hour: Fleet says no, and the card comes back in front with Fleet's why", Verb::Key("enter"), "alert", LATE),
    step("The feed Since you last looked, at the right, newest first: a glyph for each kind in its own colour, the title, what was said, and the time", Verb::Look, "region", "Since you last looked"),
    step("A row with a Job opens it", Verb::Look, "row", "Fix desktop_unit after #412, Started for the red main"),
    step("A merged pull request says its number", Verb::Look, "row", "#3413 Slot meter shows free slots"),
    answer("At 14:02 a new card joins the queue, behind the others, and Next up counts it", "arrives", "button", "Next up 5"),
    answer("Fleet will refuse the next answer", "refuse", "article", LAST),
    step("1 settles the card and Fleet says no: the card stays in front, with Fleet's why under it", Verb::Key("1"), "alert", "The Operator could not settle the card."),
    answer("Fleet's reply to the next answer will not arrive", "lose", "article", LAST),
    step("1 again: no reply in 30 seconds, so the card stays and a toast offers Retry", Verb::Key("1"), "alert", "Toast: Your answer to Rate limits per token have no default did not reach Fleet: no answer in 30 seconds"),
    step("Retry sends it again, and the card settles", Verb::Open("button", "Retry"), "article", NEXT),
    answer("The Operator switched off: nothing is queued, and the page says so with the switch", "off", "region", "The Operator is off"),
    step("Switch on is the setting Electron's Permissions tab saves, and the page reads on at once", Verb::Open("button", "Switch the Operator on"), "article", FRONT),
    step("1 settles the card, and it moves into the feed as working at once, its option as the title: the next card is in front before Fleet has said anything", Verb::Key("1"), "button", JUST_PICKED),
    step("The next card is in front", Verb::Look, "article", NEXT),
    step("The Operator's earlier card is working at the top of the feed: a turning ring in the accent, the option he picked in bold, the card's title under it, and each step it reported, a check on the done ones and the ring on the newest", Verb::Look, "button", WORKING),
    step("A press on it brings the card to the front to read: its read, every option with his pick marked and none to press, and its steps under them", Verb::Open("button", WORKING), "article", WORKING_TITLE),
    step("Esc puts the card that was in front back", Verb::Key("escape"), "article", NEXT),
    answer("The Operator reports the retry failed: the card comes back in front, with its kind chip reading Did not settle in red and a red frame, and its read starting with what he picked and why", "returns", "article", WORKING_TITLE),
    step("The keys: 1 to 4 settle the card with that option, A or Enter its pick, / Discuss, S Snooze, ↓ ↑ or j k bring the next or the last queued card, Esc leaves the box, Next up, a kept idea or a picked card, Tab moves through the page and Enter or Space presses what it is on", Verb::Look, "heading", "Operator"),
];

