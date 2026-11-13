//! The title bar walk: each step goes straight to a state the bar can be in. The scenes are
//! `titlebar/scenes.rs`; a step that names one changes the bar's data and nothing else.

use super::{press_on, right_on, scene, step, Step, Verb};

pub(super) const FIVE_WEEK: &str = "5-hour limit 42%, Weekly limit 71%";
pub(super) const WARNING: &str = "5-hour limit 62%, Weekly limit 84%";
pub(super) const DANGER: &str = "5-hour limit 97%, Weekly limit 96%";
const RESET: &str = "5-hour limit 8%, Weekly limit 3%";

/// The build switcher on Fleet's report: each scene is Fleet's JSON read as the live bar reads it.
pub static BUILD_LIVE: [Step; 11] = [
    scene("Fleet's report on Main, level with main: the restart is off and says so", "build-live-main", "button", "Fleet — Running"),
    step("The panel opens under the dot", Verb::Press, "button", "Fleet — Running"),
    scene("On Preview, as Fleet counts it: five ahead, two behind, and the branches left out say why", "build-live-preview", "list", "Left out of the preview"),
    press_on("Refresh preview is under way in the same frame, and holds until Fleet's report moves", "button", "Refresh preview", "button", "Refreshing preview"),
    scene("A restart Fleet refused says Fleet's words where a failed restart is said", "build-live-refused", "alert", "no served repository holds scripts/restart-build"),
    scene("Update to main: Fleet carries two working Jobs across the restart", "build-live-carried", "button", "Update to main"),
    press_on("Restart asks first, naming them", "button", "Update to main", "dialog", "Drones are working"),
    press_on("Yes: the restart is under way in the same frame", "button", "Restart", "button", "Updating to main"),
    step("Esc shuts the panel", Verb::Key("escape"), "button", "Fleet — Running"),
    scene("Jobs on the running build that are not kept yet are held in a bar above the screen", "build-live-trying", "region", "Trying"),
    press_on("Keep tells each Job to land; the bar goes", "button", "Keep", "button", "Fleet — Running"),
];

/// The switcher alone, on the projects Fleet serves, and how its two chips give way.
pub static SWITCHER: [Step; 9] = [
    step("The wide bar: a window narrower than 900 widens to show it", Verb::Widen(1400), "button", "Project"),
    step("Short of room the project reads whole; its Settings is in the switcher", Verb::Look, "button", "Project"),
    press_on("The switcher lists Fleet's projects, then the repositories in none", "button", "Project", "region", "Armada"),
    step("↓ moves to Storefront: its two repositories, each with its folder, and where it is kept", Verb::Key("down"), "region", "Storefront"),
    step("Esc shuts it", Verb::Key("escape"), "button", "Project"),
    press_on("Opened again, the keys are in Find", "button", "Project", "searchbox", "Find a project"),
    step("Typing finds a project by any member's name or folder", Verb::Keys(&["p", "r", "i"]), "region", "Storefront"),
    step("Enter switches the window to Storefront", Verb::Key("enter"), "button", "Project"),
    step("Wider, the repositories join the name once they fit whole", Verb::Width(1440), "button", "Project"),
];

pub static STEPS: [Step; 61] = [
    step("The wide bar: a window narrower than 900 widens to show it", Verb::Widen(1400), "button", "Project"),
    step("The bar at rest: the project with its repositories, the Places, search, the plan rings, the Operator, Fleet's dot and the menu", Verb::Look, "button", "Project"),
    step("The Places stand in a notch at the centre, the window's own brighter, and the search is an icon with its key", Verb::Look, "tablist", "Places"),
    step("⌘1 goes to the first place from wherever the keys are", Verb::Key("cmd-1"), "tab", "Jobs"),
    right_on("A right press on a place opens Hide, then View with every place; a place that must stay says Always shown", "tab", "Jobs", "menu", "Places"),
    step("Esc shuts the menu and the keys go back to the place", Verb::Key("escape"), "tab", "Jobs"),
    right_on("A right press anywhere else on the bar, here on the Operator, offers Teach under the heading Title bar", "button", "Operator", "menu", "Title bar"),
    step("Esc shuts the menu", Verb::Key("escape"), "button", "Operator"),
    press_on("The project opens the switcher: All, the projects, those in no project, and on the right the one the window is on", "button", "Project", "region", "Armada"),
    step("↓ moves to the next project, and the right side says what it holds", Verb::Key("down"), "region", "Storefront"),
    step("↑ moves back", Verb::Key("up"), "region", "Armada"),
    step("Esc shuts the switcher and the keys go back", Verb::Key("escape"), "button", "Project"),
    press_on("Opened again, the keys are in Find", "button", "Project", "searchbox", "Find a project"),
    step("Typing finds a project by any member's name or folder", Verb::Keys(&["p", "r", "i"]), "region", "Storefront"),
    step("Enter switches to it: the trigger reads the project and its repositories", Verb::Key("enter"), "button", "Project"),
    press_on("Open on Storefront, ⌘, is a step away", "button", "Project", "region", "Storefront"),
    step("⌘, opens Settings for the row the keys are on and shuts the switcher", Verb::Key("cmd-,"), "button", "Project"),
    scene("Until Fleet has a reading the rings are a sweeping glyph", "rings-loading", "img", "Analyzing Usage"),
    scene("From 95 a ring turns red; the 5-hour ring is outside, the weekly inside", "rings-danger", "button", DANGER),
    scene("An account off a subscription draws no rings", "rings-off", "button", "Fleet — Running"),
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
    scene("Swapping onto the snapshot: the window is dimmed and takes no presses, with the seconds on one banner", "build-swapping", "status", "Swapping to the build"),
    scene("A Fleet that swapped back leaves its line, with Dismiss", "build-swapped-back", "alert", "Swapped back"),
    press_on("Dismiss puts the line away", "button", "Dismiss", "button", "Fleet — Running"),
    scene("Below 900px the row keeps the rings, the dot and the menu", "narrow-window", "button", "Menu"),
    step("The menu lists what the row gave up: projects, Search, Dispatch, the Operator", Verb::Press, "button", "Menu"),
    press_on("Choosing Search leaves the menu and opens the command palette", "menuitem", "Search", "dialog", "Command palette"),
    step("Esc shuts the palette and the keys go back to the menu", Verb::Key("escape"), "button", "Menu"),
    step("Enter opens the menu again, and typing goes to Find a project", Verb::Key("enter"), "searchbox", "Find a project"),
    step("Typing finds a project in the menu", Verb::Keys(&["a", "r", "m"]), "menuitemcheckbox", "Armada"),
    step("Enter switches the window to it and shuts the menu", Verb::Key("enter"), "button", "Menu"),
    step("Enter opens the menu again, and ↑ goes to its last row, the Operator", Verb::Keys(&["enter", "up"]), "menuitem", "Operator"),
    step("Enter goes to the Operator and shuts the menu", Verb::Key("enter"), "button", "Menu"),
    scene("With no profile signed in, an amber dot stands on the menu", "signed-out", "button", "Menu"),
    step("The account entry says Sign in, and carries the dot", Verb::Press, "button", "Menu"),
    press_on("Sign in goes to the account and shuts the menu", "menuitem", "Sign in", "button", "Menu"),
    scene("Signed in again at full width, the row is whole", "back-to-rest", "button", "Operator"),
    scene("Events Fleet dropped stand as an alert above the screen, with the count that will never arrive", "events-dropped", "alert", "Events were dropped before Bridge saw them"),
    press_on("Noted puts it away until more are dropped", "button", "Noted", "button", "Operator"),
    scene("Fleet starting: Armada's mark pulses", "fleet-starting", "button", "Fleet — Starting"),
    scene("Fleet gone: the dot turns red under the Fleet panel; the panel is the fleet-panel walk's", "fleet-down", "button", "Fleet — Not running"),
    step("The panel says why", Verb::Press, "button", "Fleet — Not running"),
];
