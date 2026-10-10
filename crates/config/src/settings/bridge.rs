//! What Bridge keeps of a person's own way of looking: drawn by Bridge's own
//! panes, never by the generic form. Fleet holds them so they survive a
//! relaunch and reach every window.

use super::{Applies, Entry, Flag, Json, Kind, Section, Shipped, Words};

pub const THEME: Words = Words("bridge.theme");
pub const LAYOUT: Json = Json("bridge.layout");
pub const KEY_BINDINGS: Json = Json("bridge.keyBindings");
pub const WHERE_THINGS_ARE_OPEN: Flag = Flag("bridge.whereThingsAreOpen");

/// The theme nobody has chosen away from.
pub const SHIPPED_THEME: &str = "dark";

pub(super) const BRIDGE: &[Entry] = &[
    // Fleet keeps the word and Bridge decides what it names, so a theme naming
    // nothing is Bridge falling back to the default.
    Entry {
        key: THEME.0,
        section: Section::Bridge,
        title: "Theme",
        description: "The theme Bridge draws with: a shipped theme's id or a mod's name.",
        kind: Kind::Text,
        shipped: Shipped::Text(SHIPPED_THEME),
        applies: Applies::Live,
        row: None,
        env: None,
    },
    Entry {
        key: LAYOUT.0,
        section: Section::Bridge,
        title: "Layout",
        description: "Your own layout, as a layout.json that Settings, Layout wrote. It outranks every layout mod.",
        kind: Kind::Json,
        shipped: Shipped::Nothing,
        applies: Applies::Live,
        row: None,
        env: None,
    },
    Entry {
        key: KEY_BINDINGS.0,
        section: Section::Bridge,
        title: "Key bindings",
        description: "Your own keys for Bridge's acts, in place of the ones it ships.",
        kind: Kind::Json,
        shipped: Shipped::Nothing,
        applies: Applies::Live,
        row: None,
        env: None,
    },
    Entry {
        key: WHERE_THINGS_ARE_OPEN.0,
        section: Section::Bridge,
        title: "Where things are, open",
        description: "Job detail's Where things are chapter opens expanded.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(false),
        applies: Applies::Live,
        row: Some("job-detail-where-things-are-open"),
        env: None,
    },
];
