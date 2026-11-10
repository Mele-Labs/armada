//! The Operator's page walk: coming back at 13:05, a Yes that opens the go-ahead to file, Later, a move
//! by the keys and by a drag, Ask, the digest's tiles and Every rule, a new item at 14:02, and the
//! Operator off and switched on. Fleet is the mock in `operator/scenes.rs`; a step that names a scene is
//! Fleet speaking first. The key map is the last step's.

use super::{answer, step, Step, Verb};

const NOTE: &str = "File your note: the slot meter flickers when a Job ends";
const TOP: &str = "Since you left, the red main's fix is up as #3414, waiting on ci. Quick dispatch's plan is still the one two Jobs wait on.";

pub static STEPS: [Step; 35] = [
    step("Coming back at 13:05: the day, and the pills say what stands. storefront's main is red", Verb::Look, "status", "storefront main is red"),
    step("The Operator's one line on what matters first, the thing it is about in its colour", Verb::Look, "status", TOP),
    step("On it says what the Operator is reading now", Verb::Look, "status", "On it"),
    step("One item is open: its kind, its Job, what it asks and My read. A plan takes Approve, or Send changes in its Job", Verb::Look, "button", "Send changes"),
    step("A says Yes to the pick at once, and the go-ahead to file opens next: his note, and when he left it", Verb::Key("a"), "article", "File your note as an issue"),
    step("What you answered stays below, struck through, with what came of it", Verb::Look, "row", "Quick dispatch, done"),
    step("A go-ahead to file names its Then, with My pick marked", Verb::Look, "radio", "File and hand to Slot pool controls"),
    step("2 picks the second Then", Verb::Key("2"), "radio", "File and start a Job"),
    step("H is Always here, ⇧A Always, D No: a rule is kept only where Fleet can do the pick alone", Verb::Look, "button", "Always here"),
    step("L puts it Later, last for today, and the next item opens", Verb::Key("l"), "article", "Rework the Map legend"),
    step("Its row says Later today", Verb::Look, "row", NOTE),
    step("↓ opens the next item. Its row stays in the list, marked, second", Verb::Key("down"), "row", "Fix desktop_unit after #412"),
    step("⌥↑ moves it up a place: its row is first now, and the keys stay on it", Verb::Key("alt-up"), "row", "Fix desktop_unit after #412"),
    step("A row's grip drags it: the go-ahead lifted over Rework the Map legend", Verb::Lift(NOTE), "row", "Rework the Map legend"),
    step("Let go, it takes that place, and Fleet keeps the order", Verb::Release, "row", NOTE),
    step("/ asks the Operator about the open item: the chat stands where the digest was, the keys in its box", Verb::Key("/"), "textbox", "Ask the Operator"),
    step("Typing in the box is only typing: a, l, j, k, h, d and 2 press nothing", Verb::Typed("Can a later walk jump 2 places, like the fix did?"), "textbox", "Ask the Operator"),
    step("Enter sends it, and the Operator is reading", Verb::Key("enter"), "status", "The Operator is reading"),
    answer("Its answer comes into the chat", "said", "listitem", "Operator: It holds up the second Job's fix, and the walk can wait an hour."),
    step("Esc goes back to the page, the digest back and the keys on the open item", Verb::Key("escape"), "article", "Fix desktop_unit after #412"),
    step("Landed opens on a press, its first few rows one line each", Verb::Open("button", "Landed"), "listitem", "#3413 Slot meter shows free slots"),
    step("The rest are a press away", Verb::Look, "button", "Show all Landed"),
    step("A press on its head folds it again", Verb::Open("button", "Landed"), "button", "What I handled"),
    step("What I handled says what each act was about, and ends in the whole Log", Verb::Look, "listitem", "Started: Fix desktop_unit after #412"),
    step("A press on a time reads every time as its age, as the thread's do", Verb::Open("button", "Time 13:02:00"), "button", "Time 3m ago"),
    step("Learned shows its first few rules, and Every rule opens them all", Verb::Open("link", "Every rule"), "dialog", "Every rule"),
    step("Everywhere first, then each project. ↓ and ↑ move through them", Verb::Key("down"), "listitem", "Link a report to the open issue it names"),
    step("Esc shuts the sheet, and the keys are back on Every rule", Verb::Key("escape"), "link", "Every rule"),
    answer("At 14:02 a new item joins the queue, at its end", "arrives", "row", "Slot pool controls"),
    answer("The Operator switched off: nothing is queued, and the page says so with the switch", "off", "region", "The Operator is off"),
    step("What I handled is what it did before it was switched off", Verb::Look, "button", "What I handled"),
    step("Switch on is the setting Electron's Permissions tab saves, and the page reads on at once", Verb::Open("button", "Switch the Operator on"), "status", "On it"),
    step("Tab reaches the pills, the tiles and the links, and Enter presses the one it is on", Verb::Key("tab"), "status", "storefront main is red"),
    step("Your turn is back as Fleet keeps it", Verb::Look, "article", "Quick dispatch"),
    step(
        "The keys: ↓ ↑ or j k move through Your turn; A Yes, H Always here, ⇧A Always, D No, C Send changes; 1–3 a Then; L Later; / Ask; ⌥↑ ⌥↓ move the item; Tab and Enter the pills, tiles and links; Esc leaves the chat or the sheet",
        Verb::Look,
        "heading",
        "Your turn",
    ),
];
