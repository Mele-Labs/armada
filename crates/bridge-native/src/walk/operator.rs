//! The Operator page's walk: one card in front with its options, Discuss with a tag in the box, the talk under the card with
//! its About row, the card folded while the talk overflows and a card the Operator raises in it, Next up and a kept idea
//! brought from it, Snooze, the feed, a card arriving, a refused and a lost answer, the Operator off in the card's place and
//! switched on, the status card where nothing needs him: working, its session opened, watching, stopped and restarted, and
//! a card that comes while the card is folded, waiting in the new-card strip, a Job's plan read in the Review panel over the
//! page with the Operator's report page listed in it, a Job's build to try, and a question sent with no tag, which the
//! Operator answers in the talk. Fleet is the mock in `operator/scenes.rs`; a
//! step that names a scene is Fleet speaking first. The key map is the last step's.

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
/// His words in the talk, and the card the walk's Fleet brings while the card is folded.
const ASKED: &str = "Why hold Pocket and not the other Job? Is a 2 hour wait worse?";
const COMES: &str = "Checkout's coupon fix has a walk for you";
/// Fleet's why for a Snooze whose time has passed, as the scene `late` has it refuse.
const LATE: &str = "That time has passed. Pick a later one.";
/// The Job whose plan comes in front.
const PLAN_JOB: &str = "Fix the merge line freshness chip";
/// His question to the Operator with no tag: a talk about no card.
const QUESTION: &str = "Which Jobs are running now?";

pub static STEPS: [Step; 64] = [
    step("The head: Operator, the red main as a pill, and at the right what the Operator does unasked, by project, since the window is on All", Verb::Look, "button", ACTS),
    step("Its menu has each project as a header with its three values under it: Act then tell, Ask before starting or Only when told, ticked where set", Verb::Open("button", ACTS), "menuitemcheckbox", "Ask before starting"),
    step("A press sets that project's value at once", Verb::Open("menuitemcheckbox", "Ask before starting"), "button", ACTS),
    step("One card is in front: its kind, its title, a chip for each Job it names and its repository", Verb::Look, "article", FRONT),
    step("My read says what it found. The options are rows: the key at the left, the cost under each, and My pick on the first", Verb::Look, "button", HOLD),
    step("A hover lifts a row with an accent border and shows Choose at its right. The cost is in its tooltip too", Verb::Hover, "button", BOTH),
    step("Snooze puts the card back until a time, and Discuss talks to the Operator about it", Verb::Look, "button", "Discuss"),
    step("Discuss puts the keys in the box under the card, with a tag naming the card. The card stays in sight", Verb::Open("button", "Discuss"), "group", TAG),
    step("Typing is only typing: a, l, / and 2 press nothing", Verb::Typed(ASKED), "textbox", "Message"),
    step("Enter sends it as a talk about the card. The talk under the card overflows its room, so the card folds to one line: its title, the Operator's last act and a chevron", Verb::Key("enter"), "button", "Unfold Two Jobs are changing paging.rs"),
    step("A row About: and the card's title stands where his words turn to the card", Verb::Look, "separator", "About: Two Jobs are changing paging.rs"),
    step("His words follow it at the right, in the bubble a Job's thread draws them in", Verb::Look, "listitem", "You: Why hold Pocket and not the other Job? Is a 2 hour wait worse?"),
    answer("The Operator answers in its thread and raises a card in the talk. The card it raised is whole at the talk's end, which the talk follows. The box stays at the foot, with the keys in it", "said", "article", RAISED),
    answer("Scrolled up, the Operator's words stand above the card, under its name", "talk-up", "listitem", "Operator: Pocket's change is small. Holding it for 20 minutes costs less than the conflict. The other way is to move it into Jobs screens, which takes one Job off the files."),
    step("Esc gives the keys to the page", Verb::Key("escape"), "article", RAISED),
    step("2 settles the card in play with its second option at once. The talk stays, and the next card comes in front folded, as the talk still overflows", Verb::Key("2"), "button", "Unfold Main is red at 4c1d2e9"),
    step("A press opens it above the talk. It stays open until the talk grows or he sends again", Verb::Open("button", "Unfold Main is red at 4c1d2e9"), "article", NEXT),
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
    answer("The Operator switched off: nothing is queued, and its card stands in the front card's place, at its width: The Operator is off, an Off chip, and Switch on", "off", "region", "The Operator is off"),
    step("Switch on is the setting Electron's Permissions tab saves, and the page reads on at once", Verb::Open("button", "Switch the Operator on"), "article", FRONT),
    step("1 settles the card, and it moves into the feed as working at once, its option as the title: the next card is in front before Fleet has said anything", Verb::Key("1"), "button", JUST_PICKED),
    step("The next card is in front", Verb::Look, "article", NEXT),
    step("The Operator's earlier card is working at the top of the feed: a turning ring in the accent, the option he picked in bold, the card's title under it, and each step it reported, a check on the done ones and the ring on the newest", Verb::Look, "button", WORKING),
    step("A press on it brings the card to the front to read: its read, every option with his pick marked and none to press, and its steps under them", Verb::Open("button", WORKING), "article", WORKING_TITLE),
    step("Esc puts the card that was in front back", Verb::Key("escape"), "article", NEXT),
    answer("The Operator reports the retry failed: the card comes back in front, with its kind chip reading Did not settle in red and a red frame, and its read starting with what he picked and why", "returns", "article", WORKING_TITLE),
    answer("Nothing needs him: the page empties and the Operator is in a turn. A status card stands where the card was: Nothing needs you, a green Working chip, what it is doing and for how long, and the last act under it", "quiet", "status", "Reading the live Jobs, for 7m"),
    step("See what it is doing opens the Operator's session as a Job's view, with its thread", Verb::Open("button", "See what it is doing"), "row", "Thread: Operator"),
    step("Esc gives the keys to the Jobs list as it does in any Job, and Esc again comes back to the Operator's page and its status card", Verb::Keys(&["escape", "escape"]), "status", "Reading the live Jobs, for 7m"),
    answer("Between turns the chip reads Watching in the accent and the line says when Fleet last heard from it", "watching", "status", "Between turns. Last heard from 14m ago"),
    answer("Its session stopped: its card comes in front of any queued card, with a red frame and edge and a red Stopped chip, the title says how long ago, and the line says why. The talk and the box dim until it is back", "stopped", "status", "Its process ended"),
    step("Restart switches the Operator off and on, as Fleet ends its session with the switch and starts one on the next tick. The card reads Starting, with no button", Verb::Open("button", "Restart the Operator"), "status", "Fleet starts its session on the next tick"),
    answer("The Operator says more in the talk: the talk grows, and the card in front folds to one line again", "replies", "button", "Unfold Nothing needs you"),
    answer("A card comes where nothing was in front while the card is folded: it does not take the front. A strip over the talk holds it, with a bell, how many came, its title and Show. It glows once, then holds", "front-arrives", "region", "New card: Checkout's coupon fix has a walk for you"),
    step("⌘Enter, as Show, brings the newest card in front, unfolded, and the strip goes", Verb::Key("cmd-enter"), "article", COMES),
    answer("A Job's plan comes in front. Its one row is Review, and My read says what the Operator found", "plan-comes", "button", "Review"),
    step("A press opens the Review panel over the page with the plan chosen. What the Operator read of it, and its pick, stand above the plan", Verb::Open("button", "Review"), "group", "Operator"),
    step("The panel's list heads the Operator apart from the Jobs. Its report page only informs, so the list does not hold it", Verb::Look, "heading", "Operator"),
    step("Esc shuts the panel and gives the keys back to the plan's card", Verb::Key("escape"), "article", PLAN_JOB),
    answer("A Job's build to try comes in front: the card its thread draws, with Looks good, Needs rework, Show me how and Remind me", "build-comes", "button", "Looks good"),
    step("A press puts the keys in the box. It holds no tag", Verb::Press, "textbox", "Message"),
    step("He asks the Operator a question", Verb::Typed(QUESTION), "textbox", "Message"),
    step("Enter sends it as a talk to the Operator about no card: his words join the talk with no About row, and the card in front stays as it is", Verb::Key("enter"), "listitem", "You: Which Jobs are running now?"),
    answer("The Operator answers in the talk. The talk now overflows its room, so the card in front folds to one line, as when the Operator says more", "said", "listitem", "Operator: Pocket, Jobs screens and the desktop_unit fix are running. None of them waits on you."),
    step("The keys: 1 to 4 settle the card with that option, A or Enter its pick, / Discuss, S Snooze, ↓ ↑ or j k bring the next or the last queued card, Esc leaves the box, Next up, a kept idea or a picked card, ⌘Enter shows a card waiting in the strip, Tab moves through the page and Enter or Space presses what it is on", Verb::Look, "heading", "Operator"),
];

