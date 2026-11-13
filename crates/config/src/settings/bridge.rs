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
pub const OPEN_GROUP: Flag = Flag("bridge.openGroup");
/// Compact, Default or Roomy: which set of space tokens Bridge draws with (`packages/tokens/src/spacing-sets.css`).
pub const SPACING: Words = Words("spacing.density");
/// The thread's look: agent name, times, text size, spacing, quick replies, voice and quick responses, as one JSON value.
pub const THREAD_LOOK: Json = Json("thread.look");
pub const SPACINGS: &[&str] = &["compact", "default", "roomy"];

// How he works with Jobs. `ipc::Preferences` carries each, and `ipc` spells the same words for the closed sets.
pub const TAKE_IN: Words = Words("bridge.takeIn");
pub const ANSWER_WITH: Words = Words("bridge.answerWith");
pub const FIRST_RUN_ANSWERED: Flag = Flag("bridge.firstRunAnswered");
pub const READ_RESULTS: Words = Words("bridge.readResults");
pub const LOOK_RESULTS: Words = Words("bridge.lookResults");
pub const LOOK_QUESTIONS: Words = Words("bridge.lookQuestions");
pub const LISTEN_END_SUMMARY: Flag = Flag("bridge.listenEndSummary");
pub const LISTEN_NEEDS_YOU: Flag = Flag("bridge.listenNeedsYou");
pub const LISTEN_PROGRESS: Flag = Flag("bridge.listenProgress");
pub const LISTEN_OPERATOR: Flag = Flag("bridge.listenOperator");
pub const LISTEN_VOICE: Words = Words("bridge.listenVoice");
pub const BUTTONS_WORDS: Flag = Flag("bridge.buttonsWords");
pub const TAKE_INS: &[&str] = &["read", "look", "listen"];
pub const ANSWER_WITHS: &[&str] = &["type", "talk", "buttons"];
pub const READ_RESULTS_SET: &[&str] = &["note_then_page", "always_page", "thread_only"];
pub const LOOK_RESULTS_SET: &[&str] = &["picture_every", "picture_screens", "walk_visible"];
pub const LOOK_QUESTIONS_SET: &[&str] = &["pictures_mix", "pictures_whole"];

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
    // One value Bridge owns, for the Aa panel in a Job's head and System › Thread to read and write alike. Fleet reads four fields of it for the quick bar (`session_quick.rs`): `learn`, `quickOff`, `quickOffFor`, and the lines of `quick` and `forgotten`.
    Entry {
        key: THREAD_LOOK.0,
        section: Section::Thread,
        title: "Thread look",
        description: "How a Job's thread reads and what its quick bar offers, as JSON the Aa panel and System, Thread wrote.",
        kind: Kind::Json,
        shipped: Shipped::Nothing,
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
    Entry {
        key: OPEN_GROUP.0,
        section: Section::Bridge,
        title: "Open group",
        description: "A Job's story ends with the pull requests, issues and follow-ups still open. A right press on the group hides it.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: TAKE_IN.0,
        section: Section::Bridge,
        title: "Take work in",
        description: "What you take a Job's work in: read it, look at it or listen to it.",
        kind: Kind::Choice(Options::Fixed(TAKE_INS)),
        shipped: Shipped::Text("read"),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: ANSWER_WITH.0,
        section: Section::Bridge,
        title: "Answer with",
        description: "How you answer a Job's question: type, talk or press a button.",
        kind: Kind::Choice(Options::Fixed(ANSWER_WITHS)),
        shipped: Shipped::Text("type"),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: FIRST_RUN_ANSWERED.0,
        section: Section::Bridge,
        title: "First run answered",
        description: "You have been asked how you work with Jobs.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(false),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: READ_RESULTS.0,
        section: Section::Bridge,
        title: "Read results",
        description: "Where a Job's result goes when you read: a note in the thread then the page, the page every time, or the thread alone.",
        kind: Kind::Choice(Options::Fixed(READ_RESULTS_SET)),
        shipped: Shipped::Text("note_then_page"),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: LOOK_RESULTS.0,
        section: Section::Bridge,
        title: "Look results",
        description: "How a Job's result shows when you look: a picture of every change, of every screen, or the walk of what is visible.",
        kind: Kind::Choice(Options::Fixed(LOOK_RESULTS_SET)),
        shipped: Shipped::Text("picture_every"),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: LOOK_QUESTIONS.0,
        section: Section::Bridge,
        title: "Look questions",
        description: "How a Job's question offers its options when you look: pictures to mix parts of, or whole pictures.",
        kind: Kind::Choice(Options::Fixed(LOOK_QUESTIONS_SET)),
        shipped: Shipped::Text("pictures_mix"),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: LISTEN_END_SUMMARY.0,
        section: Section::Bridge,
        title: "Listen: end summary",
        description: "A Job that ends is read out in a summary when you listen.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: LISTEN_NEEDS_YOU.0,
        section: Section::Bridge,
        title: "Listen: needs you",
        description: "A Job that needs you is read out when you listen.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: LISTEN_PROGRESS.0,
        section: Section::Bridge,
        title: "Listen: progress",
        description: "A Job's progress is read out when you listen.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(false),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: LISTEN_OPERATOR.0,
        section: Section::Bridge,
        title: "Listen: Operator",
        description: "The Operator's words are read out when you listen.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(false),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: LISTEN_VOICE.0,
        section: Section::Bridge,
        title: "Listen: voice",
        description: "The voice you listen with, by the name the Mac gives it. Blank is the Mac's default voice.",
        kind: Kind::Text,
        shipped: Shipped::Text(""),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: BUTTONS_WORDS.0,
        section: Section::Bridge,
        title: "Buttons: words",
        description: "A text box shows under a question's options.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
];
