//! The Operator's page walk: coming back at 13:05, a Yes that opens the go-ahead to file, Later, a reorder,
//! Ask, and a new item at 14:02. Fleet is the mock in `operator/scenes.rs`; a step that names a scene is
//! Fleet speaking first. The key map is the last step's.

use super::{answer, step, Step, Verb};

pub static STEPS: [Step; 21] = [
    step("Coming back at 13:05: the day, and the pills say what stands. storefront's main is red", Verb::Look, "status", "storefront main is red"),
    step("The Operator's one line on what matters first", Verb::Look, "status", "Top line"),
    step("On it says what the Operator is reading now", Verb::Look, "status", "On it"),
    step("One item is open: its kind, its Job, what it asks and My read, with the pick filled", Verb::Look, "article", "Quick dispatch"),
    step("A says Yes to the pick at once, and the go-ahead to file opens next", Verb::Key("a"), "article", "File your note: the slot meter flickers when a Job ends"),
    step("What you answered stays below, struck through, with what came of it", Verb::Look, "row", "Quick dispatch, done"),
    step("A go-ahead to file names its Then, with My pick marked", Verb::Look, "radio", "File and hand to Slot pool controls"),
    step("2 picks the second Then", Verb::Key("2"), "radio", "File and start a Job"),
    step("Always here and Always keep the answer as a rule; they show only where Fleet can do the pick alone", Verb::Look, "button", "Always here"),
    step("L puts it Later, last for today, and the next item opens", Verb::Key("l"), "article", "Rework the Map legend"),
    step("Its row says Later today", Verb::Look, "row", "File your note: the slot meter flickers when a Job ends"),
    step("↓ opens the next item", Verb::Key("down"), "article", "Fix desktop_unit after #412"),
    step("⌥↑ moves it up a place, and the keys stay on it. A drag on a row's grip does the same", Verb::Key("alt-up"), "article", "Fix desktop_unit after #412"),
    step("/ asks the Operator about it: the chat stands where the digest was, the keys in its box", Verb::Key("/"), "textbox", "Ask the Operator"),
    step("Type a question", Verb::Typed("Why does this come first?"), "textbox", "Ask the Operator"),
    step("Enter sends it, and the Operator is reading", Verb::Key("enter"), "status", "The Operator is reading"),
    answer("Its answer comes into the chat", "said", "listitem", "Operator: It holds up the second Job's fix, and the walk can wait an hour."),
    step("Esc goes back to the page and the digest, keys and all", Verb::Key("escape"), "region", "Since you last looked"),
    answer("At 14:02 a new item joins the queue, at its end. Its toast is the shell's", "arrives", "row", "Slot pool controls"),
    step("What I handled ends in the whole Log; Landed, Running and Coming up open on a press", Verb::Look, "link", "The whole Log"),
    step(
        "The keys: ↓ ↑ or j k move through Your turn, A is Yes, 1–9 the options or the Thens, L is Later, / is Ask, ⌥↑ ⌥↓ move the item, Esc leaves the chat",
        Verb::Look,
        "heading",
        "Your turn",
    ),
];
