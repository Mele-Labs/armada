//! The sidebar's groups and entries, and the words a search finds each by: a port of Classic's
//! `system-entries.ts` and the section words in `packages/surfaces/settings/src/sections.ts`.
//!
//! **Only what is ported is listed** (the orchestrator, 12 Nov 2026): an entry Classic has that this
//! build does not draw yet is left out rather than shown blank. Each port adds its entry here.

use ipc::{Setting, SettingKind, SettingsList};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// The project Settings was opened on, from the switcher. Only while scoped.
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
    /// The project's own values (`GET /projects/values`).
    ProjectValues,
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
    Section { id: "store", label: "Store", palette: "Store address", words: &["Store address", "store", "central store", "sign in", "url"] },
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
    Section { id: "spacing", label: "Spacing", palette: "Spacing", words: &["Spacing", "density", "compact", "default", "roomy", "room", "dense"] },
];

/// Every entry this build draws, in Classic's order within each group.
pub const ENTRIES: &[Entry] = &[
    Entry { id: "project:values", label: "Values", group: Group::Project, body: Body::ProjectValues },
    Entry { id: "kit:workflows", label: "Workflows", group: Group::Kit, body: Body::Workflows },
    Entry { id: "kit:models", label: "Models", group: Group::Kit, body: Body::Models },
    Entry { id: "kit:prompts", label: "Prompts", group: Group::Kit, body: Body::Schema(&["prompts"]) },
    Entry { id: "kit:harness", label: "Harness", group: Group::Kit, body: Body::Schema(&["harness"]) },
    Entry { id: "fleet:limits", label: "Limits", group: Group::Fleet, body: Body::Schema(&["limits"]) },
    Entry { id: "fleet:timeouts", label: "Timeouts", group: Group::Fleet, body: Body::Schema(&["timeouts"]) },
    Entry { id: "fleet:retention", label: "Retention", group: Group::Fleet, body: Body::Schema(&["retention"]) },
    Entry { id: "fleet:plan", label: "Plan", group: Group::Fleet, body: Body::Schema(&["plan"]) },
    Entry { id: "fleet:token-use", label: "Token use", group: Group::Fleet, body: Body::Schema(&["token-use"]) },
    Entry { id: "fleet:machine", label: "Machine", group: Group::Fleet, body: Body::Schema(&["machine"]) },
    Entry { id: "fleet:account", label: "Account", group: Group::Fleet, body: Body::Schema(&["account"]) },
    Entry { id: "fleet:store", label: "Store", group: Group::Fleet, body: Body::Schema(&["store"]) },
    Entry { id: "fleet:features", label: "Features", group: Group::Fleet, body: Body::Features },
    Entry { id: "fleet:repository", label: "Repository", group: Group::Fleet, body: Body::Repository },
    Entry { id: "bridge:all", label: "All", group: Group::Mods, body: Body::Mods(None) },
    Entry { id: "bridge:theme", label: "Theme", group: Group::Mods, body: Body::Theme },
    Entry { id: "bridge:layout", label: "Layout", group: Group::Mods, body: Body::Layout },
    Entry { id: "bridge:words", label: "Words", group: Group::Mods, body: Body::Mods(Some(ipc::ModKind::Words)) },
    Entry { id: "bridge:tracker", label: "Tracker", group: Group::Mods, body: Body::Mods(Some(ipc::ModKind::Tracker)) },
    Entry { id: "bridge:fleet", label: "Fleet", group: Group::Mods, body: Body::Mods(Some(ipc::ModKind::Fleet)) },
    Entry { id: "bridge:spacing", label: "Spacing", group: Group::Bridge, body: Body::Schema(&["spacing"]) },
    Entry { id: "bridge:keyboard", label: "Keyboard shortcuts", group: Group::Bridge, body: Body::Keyboard },
    Entry { id: "bridge:time", label: "Time", group: Group::Bridge, body: Body::Time },
    Entry { id: "bridge:editor", label: "Editor", group: Group::Bridge, body: Body::Schema(&["editor"]) },
    Entry { id: "bridge:terminal", label: "Terminal", group: Group::Bridge, body: Body::Schema(&["terminal"]) },
    Entry { id: "bridge:links", label: "Links", group: Group::Bridge, body: Body::Schema(&["links"]) },
    Entry { id: "bridge:macs", label: "Macs", group: Group::Bridge, body: Body::Macs },
];

pub const GROUPS: [Group; 5] = [Group::Project, Group::Kit, Group::Fleet, Group::Mods, Group::Bridge];

/// ⌘, opens the entry last used, else Limits, as Classic's Settings surface does (`entryForSurface`).
pub const FIRST: &str = "fleet:limits";

pub fn entry(id: &str) -> Option<&'static Entry> {
    ENTRIES.iter().find(|one| one.id == id)
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
        Body::Repository | Body::ProjectValues | Body::Keyboard | Body::Workflows => &[],
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

/// The entries the sidebar lists for a search: the Project group only while Settings is scoped to one.
pub fn visible(query: &str, list: Option<&SettingsList>, scoped: bool) -> Vec<&'static Entry> {
    ENTRIES.iter().filter(|one| (scoped || one.group != Group::Project) && found(one, query, list)).collect()
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
        let ids = |query: &str| visible(query, Some(&list), false).iter().map(|one| one.id).collect::<Vec<_>>();
        assert_eq!(ids("timeouts"), ["fleet:timeouts"]);
        assert!(ids("drones").contains(&"fleet:limits"), "a setting's title finds its entry");
        assert!(ids("limits.memorySparePercent").contains(&"fleet:limits"), "a key finds it too");
        assert!(ids("ghostty").contains(&"bridge:terminal"), "a section's own words");
        assert!(ids("nothing at all like this").is_empty());
        assert_eq!(ids("").len(), ENTRIES.len() - 1, "every entry but the project's");
    }

    #[test]
    fn the_project_group_is_listed_only_while_scoped() {
        assert!(visible("", None, false).iter().all(|one| one.group != Group::Project));
        assert_eq!(visible("", None, true).first().map(|one| one.id), Some("project:values"));
    }

    #[test]
    fn a_search_that_hides_the_pick_draws_the_first_found_and_keeps_the_pick() {
        let list = list();
        let found = visible("ghostty", Some(&list), false);
        assert_eq!(shown("fleet:limits", &found).map(|one| one.id), Some("bridge:terminal"));
        let all = visible("", Some(&list), false);
        assert_eq!(shown("fleet:limits", &all).map(|one| one.id), Some("fleet:limits"));
    }

    #[test]
    fn json_and_the_operators_settings_are_not_drawn_by_the_form() {
        let list = list();
        assert!(list.settings.iter().any(|one| !drawn(one)), "the mock holds one the form leaves out");
        assert!(settings_in(&list, "limits").iter().all(|one| drawn(one)));
    }
}
