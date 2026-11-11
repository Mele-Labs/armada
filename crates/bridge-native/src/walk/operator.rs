//! The Operator's page walk: coming back at 13:05, a Yes that opens the go-ahead to file, Later, a move
//! by the keys and by a drag, Ask, the digest's tiles, the whole Log, Every rule and Holds with Forget, a new item at 14:02, and the
//! Operator off and switched on. Fleet is the mock in `operator/scenes.rs`; a step that names a scene is
//! Fleet speaking first, or the page scrolled to what a short window holds below the open card (`rest`,
//! `top`). The key map is the last two steps'.

use super::{answer, step, Step, Verb};

const NOTE: &str = "File your note: the slot meter flickers when a Job ends";
const HOLD: &str = "armada: only Slot pool controls writes under apps/cockpit until it pushes";
const FORGET_HOLD: &str = "Forget armada: only Slot pool controls writes under apps/cockpit until it pushes";
const TOP: &str = "Since you left, the red main's fix is up as #3414, waiting on ci. Quick dispatch's plan is still the one two Jobs wait on.";

pub static STEPS: [Step; 48] = [
    step("Coming back at 13:05: the day, and the pills say what stands. storefront's main has been red since 11:24", Verb::Look, "status", "storefront main is red since 11:24"),
    step("The Operator's one line on what changed, in grey under the day: nothing on it waits on you", Verb::Look, "status", TOP),
    step("What the Operator is reading now: the Job by name, what it asks, and its project", Verb::Look, "status", "The Operator is reading what Store reports its size asks: Review pull request #3415"),
    step("One item is open: its kind, its Job, what it asks and My read. A plan takes Approve, or Send changes in its Job", Verb::Look, "button", "Send changes"),
    step("A says Yes to the pick at once, and the go-ahead to file opens next: his note, and when he left it", Verb::Key("a"), "article", "File your note as an issue"),
    answer("What you answered stays below, struck through, with what came of it", "rest", "row", "Quick dispatch, done"),
    answer("A go-ahead to file names its Then, with My pick marked", "top", "radio", "File and hand to Slot pool controls"),
    step("2 picks the second Then", Verb::Key("2"), "radio", "File and start a Job"),
    step("H is Always here, ⇧A Always, D No: a rule is kept only where Fleet can do the pick alone", Verb::Look, "button", "Always here"),
    step("L puts it Later, last for today, and the next item opens", Verb::Key("l"), "article", "Rework the Map legend"),
    answer("Its row wears the clock: later today", "rest", "row", NOTE),
    step("↓ opens the next item. Its row stays in the list, marked, second", Verb::Key("down"), "row", "Fix desktop_unit after #412"),
    step("⌥↑ moves it up a place: its row is first now, and the keys stay on it", Verb::Key("alt-up"), "row", "Fix desktop_unit after #412"),
    step("A row's grip drags it: the go-ahead lifted over Rework the Map legend", Verb::Lift(NOTE), "row", "Rework the Map legend"),
    step("Let go, it takes that place, and Fleet keeps the order", Verb::Release, "row", NOTE),
    step("/ asks the Operator about the open item: the chat opens in its card, the keys in its box, and the digest stays", Verb::Key("/"), "textbox", "Ask the Operator"),
    step("Typing in the box is only typing: a, l, j, k, h, d and 2 press nothing", Verb::Typed("Can a later walk jump 2 places, like the fix did?"), "textbox", "Ask the Operator"),
    step("Enter sends it, and the Operator is reading", Verb::Key("enter"), "status", "The Operator is reading"),
    answer("Its answer comes into the chat", "said", "listitem", "Operator: It holds up the second Job's fix, and the walk can wait an hour."),
    step("Esc shuts the chat, and the keys are on the open item", Verb::Key("escape"), "article", "Fix desktop_unit after #412"),
    step("Landed opens on a press, its first few rows one line each", Verb::Open("button", "Landed"), "listitem", "#3413 Slot meter shows free slots"),
    step("Show all draws the rest of them", Verb::Open("button", "Show all Landed"), "listitem", "#3401 Merge strip says how fresh each line is"),
    step("Show fewer goes back to the first few", Verb::Open("button", "Show fewer Landed"), "button", "Show all Landed"),
    step("A press on its head folds Landed again", Verb::Open("button", "Landed"), "button", "Landed"),
    step("What I handled says what each act was about, and ends in the whole Log", Verb::Look, "listitem", "Started: Fix desktop_unit after #412"),
    step("It has Show all too, beside the Log", Verb::Open("button", "Show all What I handled"), "listitem", "Merged: #3406 Walk window keeps each take"),
    step("And Show fewer", Verb::Open("button", "Show fewer What I handled"), "button", "Show all What I handled"),
    step("The whole Log opens the week's journal on a sheet: each day, and each entry at its time", Verb::Open("link", "The whole Log"), "dialog", "The whole Log"),
    step("Esc shuts it, and the keys are back on The whole Log", Verb::Key("escape"), "link", "The whole Log"),
    step("A press on a time reads every time on the page as its age, as the thread's do: back, since and each row", Verb::Open("button", "Time 13:02"), "button", "Time 3m ago"),
    step("A press again reads them as clocks", Verb::Open("button", "Time 3m ago"), "button", "Time 13:02"),
    step("A press on its head folds What I handled, its aside still there", Verb::Open("button", "What I handled"), "button", "What I handled"),
    step("Learned shows its rules under each project, two lines each at most, and Every rule opens them all", Verb::Open("link", "Every rule"), "dialog", "Every rule"),
    step("Everywhere first, then each project. ↓ and ↑ move through them, the list keeping the row in view", Verb::Key("down"), "listitem", "Link a report to the open issue it names"),
    step("F forgets the rule the sheet has on, Link a report to the open issue it names: Everywhere keeps one rule, Learned five, and the next rule has the sheet", Verb::Key("f"), "listitem", "File your notes without asking"),
    step("Esc shuts the sheet, and the keys are back on Every rule", Verb::Key("escape"), "link", "Every rule"),
    step("A press on its head folds Learned, and Holds is below it", Verb::Open("button", "Learned"), "button", "Holds"),
    step("Holds, under each project: where only one Job writes, until it pushes or ends", Verb::Look, "listitem", HOLD),
    step("Forget lifts a hold, and that Job's paths are anyone's again", Verb::Open("button", "Forget storefront: only Fix desktop_unit after #412 writes under tests/unit until it ends"), "listitem", HOLD),
    step("Forget the last hold and Holds goes: no tile stands empty, and the keys are on Learned", Verb::Open("button", FORGET_HOLD), "button", "Learned"),
    answer("At 14:02 a new item joins the queue, at its end", "arrives", "row", "Slot pool controls"),
    answer("The Operator switched off: nothing is queued, and the page says so with the switch", "off", "region", "The Operator is off"),
    step("What I handled is what it did while it was on", Verb::Look, "button", "What I handled"),
    step("Switch on is the setting Electron's Permissions tab saves, and the page reads on at once", Verb::Open("button", "Switch the Operator on"), "status", "The Operator is reading what Store reports its size asks: Review pull request #3415"),
    step("Tab from the page starts at the pills, then goes through the presses, the times, the tiles and the links", Verb::Key("tab"), "status", "storefront main is red since 11:24"),
    step("Your turn is back as Fleet keeps it", Verb::Look, "article", "Quick dispatch"),
    step("The keys: ↓ ↑ or j k move through Your turn; A Yes, H Always here, ⇧A Always, D No, C Send changes; 1–3 a Then", Verb::Look, "heading", "Your turn"),
    step("L Later; / Ask; F Forget a rule or a hold; ⌥↑ ⌥↓ move the item; Enter is Yes where the page has the keys; Tab moves through the page, and Enter or Space presses what it is on; Esc leaves the chat or a sheet", Verb::Look, "heading", "Your turn"),
];

/// Away, take A of the approved sketch "Operator: walk-away mode": the setup before anything starts, the
/// band and the marks on Your turn while it answers, Back? on his return, and While you were away. Fleet
/// is the mock in `operator/scenes.rs`; `worked` is the afternoon passing, `back` his return to the window.
pub static AWAY: [Step; 19] = [
    step("Away sits at the end of the head. Pressing it starts nothing", Verb::Look, "button", "Away"),
    step("It opens what it may do while you're gone, before anything starts", Verb::Open("button", "Away"), "dialog", "While you're away"),
    step("Every project on the page is ticked; it goes away in the ones you leave ticked", Verb::Look, "checkbox", "storefront, ticked"),
    step("↓ moves through the lines and Space ticks one: Jobs already running, so it answers for the Jobs you started too", Verb::Keys(&["down", "down", "down", "space"]), "checkbox", "Jobs already running, ticked"),
    step("Its limits: until when, what it may spend, how many Jobs it starts, how much of the week. ← and → move one", Verb::Keys(&["down", "down", "right", "right"]), "group", "Until 19:00"),
    step("What it answers for you, in plain words. Plans and a pull request whose checks passed are ticked; Walks are not, so a walk waits for you", Verb::Look, "checkbox", "Walks"),
    step("What it never does, whatever is ticked", Verb::Look, "group", "Never, whatever is ticked"),
    step("Enter starts it. The band says since when, where and until when, and Come back stops it", Verb::Key("enter"), "button", "Come back"),
    step("Each item of Your turn says whether it answers it: Quick dispatch's plan, since Plans are ticked", Verb::Look, "status", "I answer this: Plans are ticked for armada: it answers once this has waited two minutes"),
    step("A walk waits for you, since Walks are not ticked", Verb::Look, "status", "Waits for you: Walks are not ticked for armada"),
    answer("The afternoon passes to 15:20: it approved the plan, landed storefront's fix, started a Job from the backlog and set an issue aside", "worked", "region", "While you were away"),
    answer("Back at the window after ten minutes behind others, it asks: Esc keeps it on, Enter is I'm back", "back", "dialog", "Back?"),
    step("I'm back: Away stops, the band goes, and what it did stays under While you were away", Verb::Key("enter"), "button", "Away"),
    step("Each answer under its project: who asked, what it chose and why. Agree keeps it; Change tells its Job what it should have done", Verb::Look, "listitem", "Quick dispatch: Approve plan version 2. It chose Approve"),
    step("Agree keeps the answer, and it leaves the list", Verb::Gone, "button", "Agree with Approve plan version 2"),
    step("Change opens a box under the answer for what it should have done", Verb::Open("button", "Change Approve its pull request"), "textbox", "What should it have done?"),
    step("He writes what it should have done", Verb::Typed("Wait for my review on storefront"), "textbox", "What should it have done?"),
    step("Enter sends it to the fix's Job, and the answer leaves the list", Verb::Key("enter"), "listitem", "Set aside: #3402 Move the theme picker"),
    step("An issue it set aside says why. Start puts a Job on it, Dismiss leaves it; with nothing left to look at, the tile goes", Verb::Gone, "button", "Start #3402"),
];
