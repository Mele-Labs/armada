//! The Operator's page walk: coming back at 13:05, a Yes that opens the go-ahead to file, Later, a move
//! by the keys and by a drag, Ask, the digest's tiles, the whole Log, Every rule and Holds with Forget, a new item at 14:02, and the
//! Operator off and switched on. Fleet is the mock in `operator/scenes.rs`; a step that names a scene is
//! Fleet speaking first, or the page scrolled to what a short window holds below the open card (`rest`,
//! `top`). The key map is the last two steps'.

use super::{answer, press_on, step, Step, Verb};

const NOTE: &str = "File your note: the slot meter flickers when a Job ends, and for a second the count reads one slot more than the pool holds before it settles";
const HOLD: &str = "armada: only Slot pool controls writes under apps/cockpit until it pushes";
const FORGET_HOLD: &str = "Forget armada: only Slot pool controls writes under apps/cockpit until it pushes";
/// Fleet's why for a Later whose time has passed, as the scene `late` has it refuse.
const LATE: &str = "Not done: 2026-11-11T14:05:00-05:00 has passed, so the item cannot wait until then";
const TOP: &str = "Since you left, the red main's fix is up as #3414, waiting on ci. Quick dispatch's plan is still the one two Jobs wait on.";

pub static STEPS: [Step; 64] = [
    step("Coming back at 13:05: the day, and the pills say what stands. storefront's main has been red since 11:24", Verb::Look, "status", "storefront main is red since 11:24"),
    step("The Operator's one line on what changed, in grey under the day: nothing on it waits on you", Verb::Look, "status", TOP),
    step("What the Operator is reading now: the Job by name, what it asks, and its project", Verb::Look, "status", "The Operator is reading what Store reports its size asks: Review pull request #3415"),
    step("One item is open: its kind, its Job, what it asks and My read. A plan takes Approve, or Send changes in its Job", Verb::Look, "button", "Send changes"),
    step("Two more Jobs raised the same ask, folded into this one: the mark beside its title names them", Verb::Look, "img", "Also raised by\nDispatch panel\nHelm composer"),
    step("A says Yes to the pick at once, and the go-ahead to file opens next: his note, and when he left it", Verb::Key("a"), "article", "File your note as an issue"),
    answer("What you answered leaves Your turn for Answered today, folded at its foot", "rest", "button", "Answered today"),
    step("A press opens it: each answer today, and what came of it", Verb::Open("button", "Answered today"), "row", "Quick dispatch, Answered Approve"),
    step("A press again folds it", Verb::Open("button", "Answered today"), "button", "Answered today"),
    answer("A go-ahead to file is one row of outcomes, each saying what happens, and My pick filled", "top", "button", "File and hand to Slot pool controls"),
    step("Its tooltip says why the Operator picks it", Verb::Hover, "button", "File and hand to Slot pool controls"),
    step("1 to 3 press an outcome, D is Don't file, and Later and Ask keep the right", Verb::Look, "button", "Don't file"),
    step("L opens Later's menu: when it comes back, each choice with its time", Verb::Key("l"), "menuitem", "Tomorrow morning"),
    step("Tomorrow morning puts it last until 09:00 tomorrow, and the next item opens", Verb::Open("menuitem", "Tomorrow morning"), "article", "Rework the Map legend"),
    step("Its pick is no option, so the button reads My pick, and the Operator's sentence is in its tooltip", Verb::Hover, "button", "My pick"),
    answer("Its row wears the clock, which says when it comes back", "rest", "img", "Later, until tomorrow 09:00"),
    step("↓ opens the next item. Its card stands where its row was, second", Verb::Key("down"), "article", "Fix desktop_unit after #412"),
    step("⌥↑ moves it up a place: its card is first now, and the keys stay on it", Verb::Key("alt-up"), "article", "Fix desktop_unit after #412"),
    step("A row's grip drags it: the go-ahead lifted over Rework the Map legend", Verb::Lift(NOTE), "row", "Rework the Map legend"),
    step("Let go, it takes that place, and Fleet keeps the order", Verb::Release, "row", NOTE),
    step("/ asks the Operator about the open item: the chat opens in its card, the keys in its box, and the digest stays", Verb::Key("/"), "textbox", "Ask the Operator"),
    step("Typing in the box is only typing: a, l, j, k, h, d and 2 press nothing", Verb::Typed("Can a later walk jump 2 places, like the fix did?"), "textbox", "Ask the Operator"),
    step("Enter sends it, and the Operator is reading", Verb::Key("enter"), "status", "The Operator is reading"),
    answer("Its answer comes into the chat", "said", "listitem", "Operator: It holds up the second Job's fix, and the walk can wait an hour."),
    step("Esc shuts the chat, and the keys are on the open item", Verb::Key("escape"), "article", "Fix desktop_unit after #412"),
    press_on("A press on a row low in the list opens its card there: nothing moves, and the keys follow it", "row", NOTE, "article", "File your note as an issue"),
    answer("Fleet will refuse the next answer", "refuse", "article", "File your note as an issue"),
    step("A sends Yes and Fleet says no: the item stays open, with Fleet's why under it", Verb::Key("a"), "alert", "Not done: the tracker did not take the new issue"),
    answer("Fleet's reply to the next answer will not arrive", "lose", "article", "File your note as an issue"),
    step("2 presses File and start a Job: no reply in 30 seconds, so the item stays open and a toast offers Retry", Verb::Key("2"), "alert", "Toast: Your answer to File your note as an issue did not reach Fleet: no answer in 30 seconds"),
    step("Retry sends it again, and the item goes to Answered today", Verb::Open("button", "Retry"), "button", "Answered today"),
    step("Opened, it says what came of it", Verb::Open("button", "Answered today"), "row", "File your note: the slot meter flickers when a Job ends, and for a second the count reads one slot more than the pool holds before it settles, Filed #3421 and started a Job"),
    step("A press again folds it", Verb::Open("button", "Answered today"), "button", "Answered today"),
    answer("Fleet will refuse the next Later, as the time it names has passed", "late", "button", "Later"),
    step("L, then Enter on 1 hour: Fleet says no, and the item comes back open with Fleet's why", Verb::Key("l"), "menuitem", "1 hour"),
    step("Fleet's why is under it", Verb::Key("enter"), "alert", LATE),
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
    step("Holds, under each project: where only one Job writes, with the paths it holds and what lifts it under the words, and a press on it opens that Job. A hold Fleet sent before it named them keeps its words alone", Verb::Look, "listitem", HOLD),
    step("Forget lifts a hold, and that Job's paths are anyone's again", Verb::Open("button", "Forget storefront: only Fix desktop_unit after #412 writes under tests/unit until it ends"), "listitem", HOLD),
    step("Forget the last hold and Holds goes: no tile stands empty, and the keys are on Learned", Verb::Open("button", FORGET_HOLD), "button", "Learned"),
    answer("At 14:02 a new item joins the queue, at its end", "arrives", "row", "Slot pool controls"),
    answer("The Operator switched off: nothing is queued, and the page says so with the switch", "off", "region", "The Operator is off"),
    step("What I handled is what it did while it was on", Verb::Look, "button", "What I handled"),
    step("Switch on is the setting Electron's Permissions tab saves, and the page reads on at once", Verb::Open("button", "Switch the Operator on"), "status", "The Operator is reading what Store reports its size asks: Review pull request #3415"),
    step("Tab from the page starts at the pills, then goes through the presses, the times, the tiles and the links", Verb::Key("tab"), "status", "storefront main is red since 11:24"),
    step("Your turn is back as Fleet keeps it", Verb::Look, "article", "Quick dispatch"),
    step("The keys: ↓ ↑ or j k move through Your turn; A Yes, H Always here, ⇧A Always, D No, C Send changes; on a go-ahead to file, 1–3 an outcome and D Don't file", Verb::Look, "heading", "Your turn"),
    step("L Later; / Ask; F Forget a rule or a hold; ⌥↑ ⌥↓ move the item; Enter is Yes where the page has the keys; Tab moves through the page, and Enter or Space presses what it is on; Esc leaves the chat or a sheet", Verb::Look, "heading", "Your turn"),
];
