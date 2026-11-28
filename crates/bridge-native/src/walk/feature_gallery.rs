//! The Features gallery walk: the areas as bands of tiles, each the feature's cover; a tile with no picture; the keys between
//! tiles; a feature opened on its pictures, newest first; Yes on an Also this?; Not this feature with a why; and back. On
//! `features::mock::gallery`.

use super::{step, Step, Verb};

pub static STEPS: [Step; 15] = [
    step("Features is a gallery: each area a band in its hue, its features as tiles, each tile its newest picture with the title under it", Verb::Widen(1100), "heading", "Running a Job"),
    step("A feature with no picture is drawn as a shape, with no words in it", Verb::Look, "img", "No picture of Pause, resume, stop, snooze"),
    step("→ moves along the tiles", Verb::Keys(&["right"]), "button", "Job list and live status"),
    step("↓ goes to the tile under it, here in the next area", Verb::Keys(&["down"]), "button", "Terminal Jobs"),
    step("↑ comes back", Verb::Keys(&["up"]), "button", "Job list and live status"),
    step("Enter opens the feature: every picture it has, newest first", Verb::Key("enter"), "heading", "Job list and live status"),
    step("The newest is the cover, which the gallery shows it by", Verb::Look, "label", "Cover"),
    step("Also this? holds what the librarian thinks may belong to it, each with Yes and Not this feature", Verb::Look, "region", "Also this?"),
    step("Yes keeps it: it joins the pictures in its place by when it was taken", Verb::PressWithin("article", "Suggested: localhost:5373/jobs?view=sidebar", "Yes"), "article", "localhost:5373/jobs?view=sidebar"),
    step("← goes to the cover", Verb::Keys(&["left"]), "article", "jobs-lamps"),
    step("Tab puts the keys on its Not this feature, and Enter asks why. The box may stay empty", Verb::Keys(&["tab", "enter"]), "textbox", "Why"),
    step("What is wrong with it", Verb::Typed("The Jobs list alone, with no live status on it"), "textbox", "Why"),
    step("Enter takes it off the feature, and the why is kept as a rule", Verb::Key("enter"), "region", "Pictures"),
    step("The next newest is the cover now", Verb::Absent("article", "jobs-lamps"), "label", "Cover"),
    step("Esc goes back to the gallery, on the tile it opened, which shows its new cover", Verb::Key("escape"), "button", "Job list and live status"),
];
