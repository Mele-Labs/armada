//! The `job-tokens` walk: the Context card with the Job's tokens over time under its rows, then the Tokens tab opened from a Job's +, its
//! header, legend and figures, and the pointer on a pause. It is a Job's own check of the screen and its source of
//! pictures; it is not shown to him.

use crate::walk::{Step, Target, Verb};

const fn step(say: &'static str, verb: Verb, role: &'static str, name: &'static str) -> Step {
    Step { id: None, say, verb, target: Target { role, name } }
}

const fn press_on(say: &'static str, role: &'static str, name: &'static str, then_role: &'static str, then: &'static str) -> Step {
    Step { id: None, say, verb: Verb::PressOn(role, name), target: Target { role: then_role, name: then } }
}

const fn marks(say: &'static str, scene: &'static str, role: &'static str, name: &'static str) -> Step {
    Step { id: None, say, verb: Verb::Marks(scene), target: Target { role, name } }
}

const JOB: &str = "Fix the merge line freshness chip";
/// The tooltip of the bucket under the pointer at the middle of the chart, on the mock Job: inside the 52 min wait. The arrow keys are held to by the tab's own tests, since the walk's keys go to the screen.
const PAUSE: &str = "12:22 to 12:27, 0% of your week, Main thread 0%, subagents 0%, Context 68k, Waited on you, 52 min. The cache went cold";

pub static STEPS: [Step; 11] = [
    marks("The Context card of a Job's ring: its rows, and under them the Job's tokens over time as bars and a context line", "ring-tokens", "img", "Tokens over time"),
    step("Open Tokens tab is the card's own press", Verb::Look, "button", "Open Tokens tab"),
    marks("The card goes, and the Jobs screen stands again", "strip-rest", "region", "Merge queue"),
    press_on("A Job is open", "row", JOB, "button", "Add tab: Done"),
    press_on("The top pane's + lists Tokens", "button", "Add tab: Done", "menuitem", "Tokens"),
    press_on("Tokens opens the graph of the Job from its start to its end", "menuitem", "Tokens", "img", "Tokens over time"),
    step("Its header: the Job's title, when it ran, for how long, and what share of the week it was", Verb::Look, "heading", JOB),
    step("What each mark means, wrapping where the pane is narrow", Verb::Look, "group", "Legend"),
    step("Four figures under the chart", Verb::Look, "group", "Figures"),
    step("The pointer over the chart puts the cursor on the bucket under it: the tooltip names the pause, how long it was and that the cache went cold", Verb::Hover, "img", "Tokens over time"),
    step("Its words: the time, the share of the week, who spent it, the context, and what the Job waited on", Verb::Look, "tooltip", PAUSE),
];
