//! What Bridge keeps of a person's own way of looking: drawn by Bridge's own
//! panes, never by the generic form. Fleet holds them so they survive a
//! relaunch and reach every window.

use super::{Applies, Entry, Flag, Json, Kind, Options, Section, Shipped, Words};

pub const THEME: Words = Words("bridge.theme");
pub const LAYOUT: Json = Json("bridge.layout");
pub const KEY_BINDINGS: Json = Json("bridge.keyBindings");
pub const TIME_FORMAT: Json = Json("bridge.timeFormat");
pub const ROW_VIEW: Json = Json("bridge.rowView");
pub const WHERE_THINGS_ARE_OPEN: Flag = Flag("bridge.whereThingsAreOpen");
/// Compact, Default or Roomy: which set of space tokens Bridge draws with (`packages/tokens/src/spacing-sets.css`).
pub const SPACING: Words = Words("spacing.density");
pub const SPACINGS: &[&str] = &["compact", "default", "roomy"];

/// The theme nobody has chosen away from.
pub const SHIPPED_THEME: &str = "dark";

pub(super) const BRIDGE: &[Entry] = &[
    Entry {
        key: SPACING.0,
        section: Section::Spacing,
        title: "Spacing",
        description: "Room around rows, panels and controls.",
        kind: Kind::Choice(Options::Fixed(SPACINGS)),
        shipped: Shipped::Text("default"),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
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
        tip: None,
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
        tip: None,
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
        tip: None,
    },
    Entry {
        key: TIME_FORMAT.0,
        section: Section::Bridge,
        title: "Time format",
        description: "How Bridge draws a time: 24-hour clock, seconds and date, as JSON Settings, Time wrote.",
        kind: Kind::Json,
        shipped: Shipped::Nothing,
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: ROW_VIEW.0,
        section: Section::Bridge,
        title: "Row view",
        description: "Which fields a Job's row in the Cockpit draws, as JSON the row menu's View group wrote.",
        kind: Kind::Json,
        shipped: Shipped::Nothing,
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
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
        tip: None,
    },
];
