//! The title bar walk: each step goes straight to a state the bar can be in. The scenes are
//! `titlebar/scenes.rs`; a step that names one changes the bar's data and nothing else.

use super::{press_on, right_on, scene, step, Step, Verb};

pub(super) const FIVE_WEEK: &str = "5-hour limit 42%, Weekly limit 71%";
pub(super) const WARNING: &str = "5-hour limit 62%, Weekly limit 84%";
pub(super) const DANGER: &str = "5-hour limit 97%, Weekly limit 96%";
const RESET: &str = "5-hour limit 8%, Weekly limit 3%";

pub static STEPS: [Step; 50] = [
    step("The bar at rest: the repository, the Places, search, the plan rings, the Operator, Fleet's dot and the menu", Verb::Look, "button", "Repository"),
    step("The Places stand in a notch at the centre: the place the window is in is brighter, and the search is an icon and its key beside it", Verb::Look, "tablist", "Places"),
    step("⌘1 goes to the first place from wherever the keys are", Verb::Key("cmd-1"), "tab", "Jobs"),
    right_on("A right press on a place opens Hide, then View with every place; a place that must stay says Always shown", "tab", "Jobs", "menu", "Places"),
    step("Esc shuts the menu and the keys go back to the place", Verb::Key("escape"), "tab", "Jobs"),
    right_on("A right press anywhere else on the bar, here on the Operator, offers Teach under the heading Title bar", "button", "Operator", "menu", "Title bar"),
    step("Esc shuts the menu", Verb::Key("escape"), "button", "Operator"),
    press_on("The repository opens its list: All, each Workspace over its repositories, the acts below", "button", "Repository", "menu", "Repository"),
    step("Typing narrows the list, and a Workspace stays over what matches", Verb::Key("s"), "searchbox", "Find a repository"),
    step("Each letter answers at once", Verb::Key("a"), "searchbox", "Find a repository"),
    press_on("Choosing a repository shuts the list and the keys go back", "menuitemcheckbox", "sample-web", "button", "Repository"),
    scene("Until Fleet has a reading the rings are a sweeping glyph", "rings-loading", "img", "Analyzing Usage"),
    scene("From 95 a ring turns red; the 5-hour ring is outside, the weekly inside", "rings-danger", "button", DANGER),
    scene("An account off a subscription draws no rings", "rings-off", "button", "Operator"),
    scene("From 80 a ring turns amber, and a bell stands on it while the step down is offered", "offer", "img", "Default to Sonnet offered"),
    press_on("The card says each limit and when it resets, and carries the offer", "button", WARNING, "button", "Default to Sonnet"),
    press_on("Accepting changes the card in the same frame", "button", "Default to Sonnet", "region", "Weekly limit"),
    scene("When the window resets the offer is to go back", "offer-reset", "status", "Weekly limit reset"),
    scene("The dot is green while Fleet runs; the panel opens under it", "rings-normal", "button", "Fleet — Running"),
    step("On Main, aligned: nothing to update", Verb::Press, "button", "Fleet — Running"),
    scene("On Preview: five ahead, two behind, and the branches left out say why", "build-preview", "list", "Left out of the preview"),
    press_on("Refresh preview is under way in the same frame", "button", "Refresh preview", "button", "Refreshing preview"),
    scene("The restart names the stage it has got to", "build-restarting", "status", "Building Fleet"),
    scene("Picked Jobs, and a snapshot ready to swap onto", "build-picked", "list", "Held by the snapshot"),
    scene("A restart that did not take says why", "build-failed", "alert", "scripts/preview stopped: cargo build failed in crates/fleet"),
    press_on("Revert says what it takes out and brings back", "button", "Revert", "list", "Jobs taken out"),
    step("Esc cancels and the panel has the keys", Verb::Key("escape"), "button", "Revert"),
    scene("A revert under way turns the button to its pending word", "build-reverting", "button", "Reverting"),
    scene("Working Drones are named before a restart adopts them", "build-drones", "button", "Update to main"),
    press_on("Restart asks first", "button", "Update to main", "dialog", "Drones are working"),
    press_on("Yes: the restart is under way in the same frame", "button", "Restart", "button", "Updating to main"),
    step("Esc shuts the panel and the keys go back to where they were", Verb::Key("escape"), "button", "Fleet — Running"),
    scene("A build the person has not kept yet is held in a bar above the screen", "build-trying", "region", "Trying"),
    press_on("Keep tells each Job to land; the bar goes", "button", "Keep", "button", "Fleet — Running"),
    scene("Below 900px the row keeps the rings, the dot and the menu", "narrow-window", "button", "Menu"),
    step("The menu lists what the row gave up: repositories, Search, Dispatch, the Operator", Verb::Press, "button", "Menu"),
    press_on("Choosing Search leaves the menu and the keys go back", "menuitem", "Search", "button", "Menu"),
    scene("With no profile signed in, an amber dot stands on the menu", "signed-out", "button", "Menu"),
    step("The account entry says Sign in, and carries the dot", Verb::Press, "button", "Menu"),
    press_on("Sign in goes to the account and shuts the menu", "menuitem", "Sign in", "button", "Menu"),
    scene("Signed in again at full width, the row is whole", "back-to-rest", "button", "Operator"),
    scene("Fleet unreachable: the dot is amber and the notice says what is shown is not live", "fleet-unreachable", "status", "Fleet unreachable"),
    scene("Fleet starting: Armada's mark pulses and no notice yet", "fleet-starting", "button", "Fleet — Starting"),
    scene("Fleet gone, inside the first 30 seconds: a red dot and no notice", "fleet-powering", "button", "Fleet — Not running"),
    scene("After that the notice stands above the screen", "fleet-down", "alert", "Fleet is not running"),
    step("The panel says why", Verb::Press, "button", "Fleet — Not running"),
    scene("A Fleet that does not match offers Restart Fleet", "fleet-mismatch", "button", "Restart Fleet"),
    press_on("Restart Fleet answers in the same frame", "button", "Restart Fleet", "button", "Restarting Fleet"),
    scene("A Fleet that came back and still does not match", "restart-apart", "alert", "Still not matching"),
    scene("No new runtime file in 30 seconds", "restart-no-answer", "alert", "Fleet did not come back inside 30 seconds"),
];
