//! The sidebar's groups and entries, and the words a search finds each by: a port of Classic's
//! `system-entries.ts` and the section words in `packages/surfaces/settings/src/sections.ts`.
//!
//! **Only what is ported is listed** (the orchestrator, 12 Nov 2026): an entry Classic has that this
//! build does not draw yet is left out rather than shown blank. Each port adds its entry here.
//!
//! **The sidebar is two groups** (the approved sketch "Native Settings: which project you're editing", 13 Nov):
//! For the project picked, every page a project layer can set, then This Mac, everything else in its order. Which
//! side a page is on is derived from the values it holds ([`side`]), never listed by hand.

use ipc::{Setting, SettingKind, SettingsList};

use crate::look::{token, Token};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// The values a project or every project sets, drawn as a grid (`project.rs`).
    Project,
    Kit,
    Fleet,
    Mods,
    Bridge,
}

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Group::Project => "Project",
            Group::Kit => "Kit",
            Group::Fleet => "Fleet",
            Group::Mods => "Mods",
            Group::Bridge => "Bridge",
        }
    }
}

/// What an entry draws beside the sidebar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    /// settings.json sections drawn from Fleet's schema, by section id (`limits`, `token-use`).
    Schema(&'static [&'static str]),
    /// The Features section, then the folders a Job's card always allows or blocks (`ReachFoldersSettings`).
    Features,
    /// One switch per repository for the connectors on his account (`tokens.connectorsOff`).
    Repository,
    /// A page of values a project, or every project, sets: every project beside the project picked (`project.rs`).
    Values(super::project::Page),
    /// Every act with a key, each key a control (`KeyboardSettings`).
    Keyboard,
    /// The theme field, then the theme mods (`ThemeSettings`, `ModList`).
    Theme,
    /// Every workflow Fleet runs, by where it is kept, each with its switch (`WorkflowManager`).
    Workflows,
    /// The models the harness offers and the model each run starts on (`KitModels`).
    Models,
    /// Each panel and place a layout arranges, with its switch, then the layout mods (`LayoutSettings`, `layout_choices`).
    Layout,
    /// How a stamp is written: three switches and a sample (`TimeSettings`, `time_format`).
    Time,
    /// How he takes a Job's work in and how he answers it: two rows of cards, each way's options under its row (`ways.rs`).
    Ways,
    /// This Mac, its Cloudflare tunnel, and the Macs and phones that reach it, off the Phone Gateway (`macs.rs`).
    Macs,
    /// A list of mods (`ModList`): All where no kind is named, else the mods of that kind, listed only while there is one.
    Mods(Option<ipc::ModKind>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: &'static str,
    pub label: &'static str,
    pub group: Group,
    pub body: Body,
    /// The registry glyph on its tile, and the state hue the tile is tinted with.
    pub icon: &'static str,
    pub hue: Token,
}

/// Where the sidebar lists an entry: under the project picked, or under This Mac.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Project,
    Mac,
}

/// A page goes under the project picked where a project layer can set anything it holds: a value of
/// `ValuesSet`, the project's share switch, or a workflow (a project's own workflows come with #3505).
pub fn side(entry: &Entry) -> Side {
    let project = !super::project::keys_of(entry.body).is_empty() || entry.body == Body::Workflows;
    if project {
        Side::Project
    } else {
        Side::Mac
    }
}

/// The sketch's six tile hues (`.c1`–`.c6`).
const RUNNING: Token = token::STATUS_RUNNING;
const SUCCESS: Token = token::STATUS_COMPLETED_SUCCESS;
const REVIEW: Token = token::STATUS_AWAITING_REVIEW;
const DELIVERY: Token = token::KIND_DELIVERY;
const ESCALATED: Token = token::STATUS_ESCALATED;
const FAILED: Token = token::STATUS_COMPLETED_FAILED;

const fn page(id: &'static str, label: &'static str, group: Group, body: Body, icon: &'static str, hue: Token) -> Entry {
    Entry { id, label, group, body, icon, hue }
}

/// One settings.json section: its id, its name, the palette's name for it, and the words that find it before
/// Fleet has answered (`SCHEMA_SECTIONS` and `PANES`).
pub struct Section {
    pub id: &'static str,
    pub label: &'static str,
    pub palette: &'static str,
    pub words: &'static [&'static str],
}

pub const SECTIONS: &[Section] = &[
    Section { id: "limits", label: "Limits", palette: "Fleet limits", words: &["Drones at once", "concurrency", "parallel", "memory", "disk", "checks at once", "ports"] },
    Section { id: "timeouts", label: "Timeouts", palette: "Timeouts", words: &["Timeouts", "time limit", "seconds", "wait", "reconnect"] },
    Section { id: "retention", label: "Retention", palette: "Retention", words: &["Retention", "keep", "days", "history", "clean up"] },
    Section { id: "machine", label: "Machine", palette: "Machine name", words: &["Machine name", "host", "computer", "name"] },
    Section { id: "account", label: "Account", palette: "Sign-in app", words: &["Sign-in app", "account", "sign in", "client id", "oauth", "forge"] },
    Section { id: "token-use", label: "Token use", palette: "Token use", words: &["Token use", "tokens", "cost", "compaction", "turn summaries", "cache", "thinking"] },
    Section { id: "harness", label: "Harness", palette: "Harness", words: &["Harness", "agent", "program", "binary", "path"] },
    Section { id: "plan", label: "Plan", palette: "Plan step down", words: &["Plan", "weekly", "limit", "downshift", "lower default", "step down", "percent"] },
    Section { id: "model", label: "Model", palette: "Models", words: &["Models", "model", "roster", "opus", "sonnet", "haiku"] },
    Section { id: "effort", label: "Effort", palette: "Effort", words: &["Effort", "reasoning", "low", "medium", "high"] },
    Section { id: "prompts", label: "Prompts", palette: "Drone prompts", words: &["Prompts", "brief", "baseline", "instructions"] },
    Section { id: "features", label: "Features", palette: "Features", words: &["Features", "switch", "on your words", "draft pull requests"] },
    Section { id: "editor", label: "Editor", palette: "Editor", words: &["Editor", "open", "code", "vscode", "visual"] },
    Section { id: "terminal", label: "Terminal", palette: "Terminal", words: &["Terminal", "iterm", "ghostty", "warp", "wezterm", "shell"] },
    Section { id: "links", label: "Links", palette: "Links", words: &["Open pull requests in", "pull request", "link", "browser", "panel"] },
    Section { id: "mods-all", label: "All", palette: "All mods", words: &["All", "mods", "applied", "every", "list", "taught", "switch", "promote"] },
    Section { id: "theme", label: "Theme", palette: "Theme", words: &["Theme", "colours", "colors", "dark", "light", "appearance", "mods"] },
    Section { id: "words", label: "Words", palette: "Words mods", words: &["Words", "mods", "teach", "taught", "jira", "links", "keys", "switch"] },
    Section { id: "trackers", label: "Tracker", palette: "Tracker mods", words: &["Tracker", "mods", "issue tracker", "tried", "valid", "stale", "try", "switch", "promote"] },
    Section { id: "fleet-mods", label: "Fleet", palette: "Fleet mods", words: &["Fleet", "mods", "teach", "taught", "notices", "batch", "quiet", "switch"] },
    Section { id: "macs", label: "Macs", palette: "Macs", words: &["Macs", "mac", "this mac", "tunnel", "cloudflare", "devices", "phones", "unpair", "address"] },
    Section { id: "layout", label: "Layout", palette: "Layout", words: &["Layout", "tabs", "panels", "rail", "hide", "order", "reset"] },
    Section { id: "time", label: "Time", palette: "Time", words: &["Time", "clock", "24-hour", "12-hour", "seconds", "date", "timestamp", "stamp", "format"] },
    Section {
        id: "ways",
        label: "Ways of working",
        palette: "Ways of working",
        words: &["Ways of working", "read", "look", "listen", "type", "talk", "buttons", "answer", "take in", "voice", "aloud", "sample", "pictures", "microphone", "whisper", "first run"],
    },
    Section { id: "spacing", label: "Spacing", palette: "Spacing", words: &["Spacing", "density", "compact", "default", "roomy", "room", "dense"] },
    Section { id: "pull-requests", label: "Pull requests", palette: "Pull requests", words: &["How work lands", "delivery", "auto land", "draft", "ready", "merging", "merge", "review model", "reviews", "plans", "walks", "project"] },
    Section { id: "starting-jobs", label: "Starting Jobs", palette: "Starting Jobs", words: &["Starting Jobs", "start at once", "ask me", "operator", "tracker", "issues", "linear", "beads", "project"] },
    Section { id: "look", label: "Look", palette: "Look", words: &["Look", "theme", "layout", "spacing", "density", "project"] },
];

/// Every entry this build draws, in Classic's order within each group.
pub const ENTRIES: &[Entry] = &[
    page("values:pull-requests", "Pull requests", Group::Project, Body::Values(super::project::Page::PullRequests), "git-merge", RUNNING),
    page("values:starting-jobs", "Starting Jobs", Group::Project, Body::Values(super::project::Page::StartingJobs), "play", SUCCESS),
    page("values:look", "Look", Group::Project, Body::Values(super::project::Page::Look), "layers", REVIEW),
    page("kit:workflows", "Workflows", Group::Kit, Body::Workflows, "git-branch", DELIVERY),
    page("kit:models", "Models", Group::Kit, Body::Models, "bot", ESCALATED),
    page("kit:prompts", "Prompts", Group::Kit, Body::Schema(&["prompts"]), "message-square", REVIEW),
    page("kit:harness", "Harness", Group::Kit, Body::Schema(&["harness"]), "cpu", ESCALATED),
    page("fleet:limits", "Limits", Group::Fleet, Body::Schema(&["limits"]), "gauge", FAILED),
    page("fleet:timeouts", "Timeouts", Group::Fleet, Body::Schema(&["timeouts"]), "hourglass", FAILED),
    page("fleet:retention", "Retention", Group::Fleet, Body::Schema(&["retention"]), "archive", DELIVERY),
    page("fleet:plan", "Plan", Group::Fleet, Body::Schema(&["plan"]), "calendar", REVIEW),
    page("fleet:token-use", "Token use", Group::Fleet, Body::Schema(&["token-use"]), "zap", RUNNING),
    page("fleet:machine", "Machine", Group::Fleet, Body::Schema(&["machine"]), "laptop", SUCCESS),
    page("fleet:account", "Account", Group::Fleet, Body::Schema(&["account"]), "lock", SUCCESS),
    page("fleet:features", "Features", Group::Fleet, Body::Features, "toggle-left", RUNNING),
    page("fleet:repository", "Repository", Group::Fleet, Body::Repository, "plug", ESCALATED),
    page("bridge:all", "All", Group::Mods, Body::Mods(None), "puzzle", ESCALATED),
    page("bridge:theme", "Theme", Group::Mods, Body::Theme, "palette", REVIEW),
    page("bridge:layout", "Layout", Group::Mods, Body::Layout, "layout-dashboard", RUNNING),
    page("bridge:words", "Words", Group::Mods, Body::Mods(Some(ipc::ModKind::Words)), "link", DELIVERY),
    page("bridge:tracker", "Tracker", Group::Mods, Body::Mods(Some(ipc::ModKind::Tracker)), "ticket", FAILED),
    page("bridge:fleet", "Fleet", Group::Mods, Body::Mods(Some(ipc::ModKind::Fleet)), "drone", SUCCESS),
    page("bridge:ways", "Ways of working", Group::Bridge, Body::Ways, "message-square", RUNNING),
    page("bridge:spacing", "Spacing", Group::Bridge, Body::Schema(&["spacing"]), "rows-3", REVIEW),
    page("bridge:keyboard", "Keyboard shortcuts", Group::Bridge, Body::Keyboard, "terminal", SUCCESS),
    page("bridge:time", "Time", Group::Bridge, Body::Time, "clock", FAILED),
    page("bridge:editor", "Editor", Group::Bridge, Body::Schema(&["editor"]), "file-pen-line", SUCCESS),
    page("bridge:terminal", "Terminal", Group::Bridge, Body::Schema(&["terminal"]), "square-terminal", FAILED),
    page("bridge:links", "Links", Group::Bridge, Body::Schema(&["links"]), "external-link", RUNNING),
    page("bridge:macs", "Macs", Group::Bridge, Body::Macs, "globe", RUNNING),
];

/// This Mac's parts, in order: the first drawn under This Mac itself, Mods and Bridge each under its own name.
pub const MAC_PARTS: [(Option<&str>, &[Group]); 3] = [(None, &[Group::Kit, Group::Fleet]), (Some("Mods"), &[Group::Mods]), (Some("Bridge"), &[Group::Bridge])];

/// ⌘, opens the entry last used, else Limits, as Classic's Settings surface does (`entryForSurface`).
pub const FIRST: &str = "fleet:limits";

pub fn entry(id: &str) -> Option<&'static Entry> {
    ENTRIES.iter().find(|one| one.id == id)
}

/// The page that holds a project's value.
pub fn holding(key: super::project::Key) -> Option<&'static Entry> {
    ENTRIES.iter().find(|one| matches!(one.body, Body::Values(page) if page.keys().contains(&key)))
}

/// A settings.json section's id from its name, as `sectionId` makes it: `Token use` is `token-use`.
pub fn section_id(section: &str) -> String {
    let mut id = String::new();
    for ch in section.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            id.push(ch);
        } else if !id.ends_with('-') {
            id.push('-');
        }
    }
    id.trim_matches('-').to_string()
}

/// A setting the generic form draws: a Json value is drawn by a pane of its own, and the Operator's are drawn on
/// its page (`drawn`, `sections.ts`).
pub fn drawn(setting: &Setting) -> bool {
    let operators = setting.key.split_once('.').is_some_and(|(_, rest)| rest.starts_with("operator"));
    !matches!(setting.kind, SettingKind::Json) && !operators
}

/// The settings one schema section holds, in Fleet's order.
pub fn settings_in<'a>(list: &'a SettingsList, section: &str) -> Vec<&'a Setting> {
    list.settings.iter().filter(|one| drawn(one) && section_id(&one.section) == section).collect()
}

/// Whether `words` hold every word of `query`, case aside (`holds`, `kit-rows.ts`).
pub fn holds(words: &str, query: &str) -> bool {
    let said = words.to_lowercase();
    query.to_lowercase().split_whitespace().all(|one| said.contains(one))
}

/// The words a search finds an entry by: its name, every word of the sections it holds, and once Fleet has read
/// settings.json each of those settings' title, key, description and section (`wordsOf`, `system-entries.ts`).
pub(super) fn words_of(entry: &Entry, list: Option<&SettingsList>) -> String {
    let sections: &[&str] = match entry.body {
        Body::Schema(ids) => ids,
        Body::Features => &["features"],
        Body::Theme => &["theme"],
        Body::Models => &["model", "effort"],
        Body::Mods(None) => &["mods-all"],
        Body::Mods(Some(ipc::ModKind::Tracker)) => &["trackers"],
        Body::Mods(Some(ipc::ModKind::Words)) => &["words"],
        Body::Mods(Some(_)) => &["fleet-mods"],
        Body::Macs => &["macs"],
        Body::Layout => &["layout"],
        Body::Time => &["time"],
        Body::Ways => &["ways"],
        Body::Values(page) => &[page.section()],
        Body::Repository | Body::Keyboard | Body::Workflows => &[],
    };
    let mut words = vec![entry.label.to_string()];
    if let Body::Keyboard = entry.body {
        words.push(super::keyboard::all_words());
    }
    for id in sections {
        if let Some(section) = SECTIONS.iter().find(|one| one.id == *id) {
            words.push(section.label.into());
            words.push(section.palette.into());
            words.extend(section.words.iter().map(|word| word.to_string()));
        }
        // Models draws the harness's list, not those rows, so their words do not find it (`wordsOf`).
        if let Some(list) = list.filter(|_| entry.body != Body::Models) {
            for one in settings_in(list, id) {
                words.extend([one.title.clone(), one.key.clone(), one.description.clone(), one.section.clone()]);
            }
        }
    }
    words.join(" ")
}

/// Whether a search finds an entry. An empty search finds every one.
pub fn found(entry: &Entry, query: &str, list: Option<&SettingsList>) -> bool {
    query.trim().is_empty() || holds(&words_of(entry, list), query)
}

/// The entries the sidebar lists for a search.
pub fn visible(query: &str, list: Option<&SettingsList>) -> Vec<&'static Entry> {
    ENTRIES.iter().filter(|one| found(one, query, list)).collect()
}

/// The entries in the order the sidebar draws them: the project's, then This Mac's part by part.
pub fn drawn_order<'a>(visible: &[&'a Entry]) -> Vec<&'a Entry> {
    let mut order: Vec<&Entry> = visible.iter().copied().filter(|one| side(one) == Side::Project).collect();
    for (_, groups) in MAC_PARTS {
        order.extend(visible.iter().copied().filter(|one| side(one) == Side::Mac && groups.contains(&one.group)));
    }
    order
}

/// The entry drawn: the one picked while the search keeps it, else the first the search found, the pick kept
/// (`shown`, `system-surface.tsx`).
pub fn shown<'a>(picked: &str, visible: &[&'a Entry]) -> Option<&'a Entry> {
    visible.iter().find(|one| one.id == picked).or_else(|| visible.first()).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> SettingsList {
        ipc::decode("SettingsList", include_bytes!("mock-settings.json")).expect("the mock settings")
    }

    #[test]
    fn a_section_is_named_as_classic_names_it() {
        assert_eq!(section_id("Token use"), "token-use");
        assert_eq!(section_id("Limits"), "limits");
    }

    #[test]
    fn a_search_finds_an_entry_by_its_name_its_words_or_a_setting_it_holds() {
        let list = list();
        let ids = |query: &str| visible(query, Some(&list)).iter().map(|one| one.id).collect::<Vec<_>>();
        assert_eq!(ids("timeouts"), ["fleet:timeouts"]);
        assert!(ids("drones").contains(&"fleet:limits"), "a setting's title finds its entry");
        assert!(ids("limits.memorySparePercent").contains(&"fleet:limits"), "a key finds it too");
        assert!(ids("ghostty").contains(&"bridge:terminal"), "a section's own words");
        assert!(ids("nothing at all like this").is_empty());
        assert_eq!(ids("").len(), ENTRIES.len(), "every entry");
        assert_eq!(ids("how work lands"), ["values:pull-requests"], "a value finds the page that holds it");
    }

    #[test]
    fn the_projects_pages_are_those_a_project_layer_sets_and_this_mac_holds_the_rest_in_order() {
        let ids = |side_of: Side| ENTRIES.iter().filter(|one| side(one) == side_of).map(|one| one.id).collect::<Vec<_>>();
        assert_eq!(ids(Side::Project), ["values:pull-requests", "values:starting-jobs", "values:look", "kit:workflows", "bridge:macs"]);
        let mac = ids(Side::Mac);
        assert_eq!(
            mac,
            [
                "kit:models", "kit:prompts", "kit:harness", "fleet:limits", "fleet:timeouts", "fleet:retention", "fleet:plan", "fleet:token-use", "fleet:machine", "fleet:account", "fleet:features",
                "fleet:repository", "bridge:all", "bridge:theme", "bridge:layout", "bridge:words", "bridge:tracker", "bridge:fleet", "bridge:ways", "bridge:spacing", "bridge:keyboard", "bridge:time", "bridge:editor",
                "bridge:terminal", "bridge:links",
            ],
            "everything else, in today's order"
        );
        let all: Vec<&str> = ENTRIES.iter().map(|one| one.id).collect();
        assert_eq!(drawn_order(&ENTRIES.iter().collect::<Vec<_>>()).iter().map(|one| one.id).collect::<Vec<_>>().len(), all.len(), "no page is dropped");
    }

    #[test]
    fn each_tile_is_a_registry_glyph() {
        for one in ENTRIES {
            assert!(gpui::AssetSource::load(&crate::assets::Assets, &crate::assets::icon_path(one.icon)).ok().flatten().is_some(), "{}: {}", one.id, one.icon);
        }
    }

    #[test]
    fn a_search_that_hides_the_pick_draws_the_first_found_and_keeps_the_pick() {
        let list = list();
        let found = visible("ghostty", Some(&list));
        assert_eq!(shown("fleet:limits", &found).map(|one| one.id), Some("bridge:terminal"));
        let all = visible("", Some(&list));
        assert_eq!(shown("fleet:limits", &all).map(|one| one.id), Some("fleet:limits"));
    }

    #[test]
    fn json_and_the_operators_settings_are_not_drawn_by_the_form() {
        let list = list();
        assert!(list.settings.iter().any(|one| !drawn(one)), "the mock holds one the form leaves out");
        assert!(settings_in(&list, "limits").iter().all(|one| drawn(one)));
    }
}
