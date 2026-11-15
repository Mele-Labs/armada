//! The Operator page's walk: one card in front with its options, Discuss with a tag in the box, the talk under the folded
//! card and a card the Operator raises in it, Waiting and its list, Discuss later, the feed, a card arriving, a refused and a
//! lost answer, and the Operator off and switched on. Fleet is the mock in `operator/scenes.rs`; a step that names a scene is
//! Fleet speaking first. The key map is the last step's.

use super::{answer, step, Step, Verb};

const FRONT: &str = "Two Jobs are changing paging.rs";
const TAG: &str = "About Two Jobs are changing paging.rs";
const RAISED: &str = "Hold Pocket for 20 minutes or move its change";
const NEXT: &str = "Main is red at 4c1d2e9";
const THIRD: &str = "Dark mode for the settings page needs a palette";
const LAST: &str = "Rate limits per token have no default";
const ACTS: &str = "What the Operator does unasked, armada: Act then tell";
/// Fleet's why for a Later whose time has passed, as the scene `late` has it refuse.
const LATE: &str = "That time has passed. Pick a later one.";

pub static STEPS: [Step; 35] = [
    step("The head: Operator, the red main as a pill, and at the right Waiting, the one count on the page, with what the Operator does unasked in armada beside it", Verb::Look, "button", "Waiting 4"),
    step("The Operator's value for the project is a menu: Act then tell, Ask before starting or Only when told", Verb::Open("button", ACTS), "menuitemcheckbox", "Ask before starting"),
    step("A press sets it at once", Verb::Open("menuitemcheckbox", "Ask before starting"), "button", "What the Operator does unasked, armada: Ask before starting"),
    step("One card is in front: its kind, its title, a chip for each Job it names and its repository", Verb::Look, "article", FRONT),
    step("My read says what it found. The options are rows: the cost under each, My pick on the first, and a number at the right", Verb::Look, "button", "Hold Pocket until Jobs screens pushes"),
    step("The cost is in its tooltip too", Verb::Hover, "button", "Let both run"),
    step("Discuss later puts the card back until a time, and Discuss talks to the Operator about it", Verb::Look, "button", "Discuss"),
    step("Discuss puts the keys in the box under the card, with a tag naming the card. The card stays in sight", Verb::Open("button", "Discuss"), "group", TAG),
    step("Typing is only typing: a, l, / and 2 press nothing", Verb::Typed("Why hold Pocket and not the other Job? Is a 2 hour wait worse?"), "textbox", "Message"),
    step("Enter sends it as a talk about the card, which folds to one line, with its chevron", Verb::Key("enter"), "button", "Unfold Two Jobs are changing paging.rs"),
    step("His words are the first of the talk, under the folded card", Verb::Look, "listitem", "You: Why hold Pocket and not the other Job? Is a 2 hour wait worse?"),
    answer("The Operator answers in its thread, and raises a card in the talk", "said", "listitem", "Operator: Pocket's change is small. Holding it for 20 minutes costs less than the conflict. The other way is to move it into Jobs screens, which takes one Job off the files."),
    step("The card it raised is whole in the talk, indented under its line. The box stays at the foot, with the keys in it", Verb::Look, "article", RAISED),
    step("Esc gives the keys to the page", Verb::Key("escape"), "article", RAISED),
    step("2 settles the card in play with its second option at once", Verb::Key("2"), "article", NEXT),
    step("Its talk is done and the next card is in front. Waiting has three", Verb::Look, "button", "Waiting 3"),
    step("Waiting opens the list of the cards queued behind: each its kind, its title and its Job or repository", Verb::Open("button", "Waiting 3"), "menuitem", THIRD),
    step("↓ moves through the rows, and Enter brings that card to the front", Verb::Keys(&["down", "down", "enter"]), "article", THIRD),
    step("L opens Discuss later's menu: when the card comes back, each choice with its time", Verb::Key("l"), "menuitem", "Tomorrow morning"),
    step("A time puts the card back, and the next one comes to the front", Verb::Open("menuitem", "Tomorrow morning"), "article", LAST),
    answer("Fleet will refuse the next Discuss later, as the time it names has passed", "late", "button", "Discuss later"),
    step("L opens the times again", Verb::Key("l"), "menuitem", "1 hour"),
    step("Enter on 1 hour: Fleet says no, and the card comes back in front with Fleet's why", Verb::Key("enter"), "alert", LATE),
    step("The feed Since you last looked, at the right, newest first: a glyph for each kind in its own colour, the title, what was said, and the time", Verb::Look, "region", "Since you last looked"),
    step("A row with a Job opens it", Verb::Look, "row", "Fix desktop_unit after #412, Started for the red main"),
    step("A merged pull request says its number", Verb::Look, "row", "#3413 Slot meter shows free slots"),
    answer("At 14:02 a new card joins the queue, behind the others", "arrives", "button", "Waiting 3"),
    answer("Fleet will refuse the next answer", "refuse", "article", LAST),
    step("1 settles the card and Fleet says no: the card stays in front, with Fleet's why under it", Verb::Key("1"), "alert", "The Operator could not settle the card."),
    answer("Fleet's reply to the next answer will not arrive", "lose", "article", LAST),
    step("1 again: no reply in 30 seconds, so the card stays and a toast offers Retry", Verb::Key("1"), "alert", "Toast: Your answer to Rate limits per token have no default did not reach Fleet: no answer in 30 seconds"),
    step("Retry sends it again, and the card settles", Verb::Open("button", "Retry"), "article", NEXT),
    answer("The Operator switched off: nothing is queued, and the page says so with the switch", "off", "region", "The Operator is off"),
    step("Switch on is the setting Electron's Permissions tab saves, and the page reads on at once", Verb::Open("button", "Switch the Operator on"), "article", FRONT),
    step("The keys: 1 to 4 settle the card with that option, A or Enter its pick, / Discuss, L Discuss later, ↓ ↑ or j k bring the next or the last waiting card, Esc leaves the box or the list, Tab moves through the page and Enter or Space presses what it is on", Verb::Look, "heading", "Operator"),
];

