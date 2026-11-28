//! The sub project goal walk: Start a sub project asks one thing, the goal; Turn into orchestrator asks the same; the sub
//! project's heading is its goal with its orchestrator the first row; and the orchestrator's Goal met? card, on the board and
//! on its own Needs you, with what landed, Agree and Not yet. On `mock::sub_projects` and the scenario's Jobs.

use super::{press_on, step, Step, Verb};
use crate::jobs::ORCHESTRATOR_SAID;
use crate::mock::SUB_GOAL;

pub static STEPS: [Step; 14] = [
    step("A right press on a Job opens its menu: Start a sub project", Verb::RightPress("Fix the merge line freshness chip"), "menuitem", "Start a sub project"),
    press_on("The sheet asks one thing, the goal, in your words. The goal names the sub project too", "menuitem", "Start a sub project", "textbox", "Goal"),
    step("He writes it; Enter starts it", Verb::Typed("Sign-in works on the phone"), "textbox", "Goal"),
    step("Esc leaves and keeps the words for next time", Verb::Key("escape"), "row", "Fix the merge line freshness chip"),
    step("The same menu turns a running Job into an orchestrator", Verb::RightPress("Fix the merge line freshness chip"), "menuitem", "Turn into orchestrator"),
    press_on("It asks the goal in the same sheet", "menuitem", "Turn into orchestrator", "dialog", "Turn into orchestrator"),
    step("The sub project heads the list under its goal", Verb::Key("escape"), "heading", SUB_GOAL),
    step("Its first row is the orchestrator, marked with the control tower", Verb::Hover, "img", ORCHESTRATOR_SAID),
    step("The Jobs it coordinates stand under it", Verb::Look, "row", "Native Bridge foundation"),
    press_on("The heading opens its board: the orchestrator asks Goal met?", "heading", SUB_GOAL, "group", "Goal met?"),
    step("What landed, a row each: number and title", Verb::Look, "listitem", "#4102 Work out thread rows in Fleet"),
    step("Agree folds the sub project away; Not yet keeps it open", Verb::Look, "button", "Agree"),
    press_on("The orchestrator's own Needs you holds the same card", "row", SUB_GOAL, "button", "Needs you"),
    step("Opened, it draws the goal, what landed, Agree and Not yet", Verb::Open("button", "Needs you"), "group", "Goal met?"),
];
