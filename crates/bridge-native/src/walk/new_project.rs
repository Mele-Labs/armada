//! The New project walk: the sheet opened from the project switcher, a name, a repository opened on its
//! folders and two ticked through its search, a third found three levels down by Fleet's search, a repository to clone, each tile, and Create, which lands on the
//! new Job's thread in the project with the keys in its box. The sheet's Fleet is `new_project::wire::Mock`.

use super::{press_on, step, Step, Verb};

/// The switcher on a wide bar, its rows in the menu on a narrow one: either way Find has the keys.
const SWITCHER: Verb = Verb::PressOnFirst(&[("button", "Project"), ("button", "Menu")]);

pub static STEPS: [Step; 36] = [
    step("The project switcher, or below 900 the menu that holds its rows: the keys are in Find", SWITCHER, "searchbox", "Find a project"),
    step("Typing narrows it to New project", Verb::Keys(&["n", "e", "w"]), "searchbox", "Find a project"),
    step("Enter opens the sheet over the window, the keys in Name", Verb::Key("enter"), "textbox", "Name"),
    step("Its name", Verb::Typed("Pocket"), "textbox", "Name"),
    step("Esc shuts the sheet", Verb::Key("escape"), "searchbox", "Search Jobs"),
    step("Opened again", SWITCHER, "searchbox", "Find a project"),
    step("New project again", Verb::Keys(&["n", "e", "w"]), "searchbox", "Find a project"),
    step("The name is as he left it", Verb::Key("enter"), "textbox", "Name"),
    step("His repositories, each with the folder it is in", Verb::Look, "tree", "Repositories"),
    press_on("A press opens one on its folders: the whole of it, a search, and its top folders", "treeitem", "sample-web", "checkbox", "Whole repository"),
    press_on("Its search reaches every folder of it", "searchbox", "Find a folder in sample-web", "searchbox", "Find a folder in sample-web"),
    step("Typing finds each folder with those letters in order, the letters marked", Verb::Typed("chko"), "checkbox", "web/checkout"),
    step("↓ takes the keys to the first folder", Verb::Key("down"), "checkbox", "web/checkout"),
    step("Space ticks it: the repository reads 1 folder picked, its box half filled", Verb::Key("space"), "treeitem", "sample-web"),
    step("↓ and Space tick the next", Verb::Keys(&["down", "space"]), "checkbox", "services/checkout-api"),
    step("2 folders picked", Verb::Look, "treeitem", "sample-web"),
    press_on("The search takes the keys again", "searchbox", "Find a folder in sample-web", "searchbox", "Find a folder in sample-web"),
    step("Cleared, the list is the top folders again, and the two ticked stay in it", Verb::Keys(&["cmd-a", "backspace"]), "checkbox", "web/checkout"),
    step("Typing asks Fleet, which finds a folder three levels down that the top folders do not show", Verb::Typed("pricrules"), "checkbox", "services/pricing/rules"),
    step("↓ and Space tick it", Verb::Keys(&["down", "space"]), "checkbox", "services/pricing/rules"),
    press_on("The search takes the keys again", "searchbox", "Find a folder in sample-web", "searchbox", "Find a folder in sample-web"),
    step("Cleared, the folder stays ticked beside the others the list does not show", Verb::Keys(&["cmd-a", "backspace"]), "checkbox", "services/pricing/rules"),
    step("3 folders picked", Verb::Look, "treeitem", "sample-web"),
    press_on("Clone a repository opens its two fields, the keys in the first", "button", "Clone a repository", "textbox", "Repository to clone"),
    step("A URL or owner and name", Verb::Typed("github.com/acme/billing"), "textbox", "Repository to clone"),
    step("It clones into the folder his repositories are in, or another he picks", Verb::Look, "button", "Folder to clone into"),
    step("Where its file is kept: this Mac, a synced folder or the cloud", Verb::Wheel("dialog", "New project", 240), "radiogroup", "Where its file is kept"),
    step("Use defaults keeps the settings folded: how work lands is asked on the first Job, and the rest is what every project uses", Verb::Wheel("dialog", "New project", 240), "radio", "Use defaults"),
    press_on("Customize opens every group. Grey is what every project uses", "radio", "Customize", "button", "How work lands"),
    step("Landing comes first", Verb::Wheel("dialog", "New project", 300), "button", "How work lands"),
    press_on("Each setting is a menu", "button", "How work lands", "menuitemcheckbox", "Auto land"),
    press_on("Picked, it is in full colour with the modified dot", "menuitemcheckbox", "Auto land", "img", "How work lands set for this project"),
    step("Sharing", Verb::Wheel("dialog", "New project", 900), "switch", "Share with your other Macs"),
    step("⌘↩ clones the repository, makes the project and goes to it: a Job starts in it on its empty thread, the keys in its box", Verb::Key("cmd-enter"), "textbox", "Message"),
    step("The line over the box says how its work lands, from the project's settings. The bar names the project and its repositories: sample-web, for its two folders, and billing, the clone", Verb::Look, "status", "Auto land, from Pocket's settings"),
    step("Its row in the list", Verb::Look, "row", "s-7c41e2a9"),
];
