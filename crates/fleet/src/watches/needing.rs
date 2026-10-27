//! A watch that ended `not_ours`, on its Job's Needs you list: Fleet raises it, the agent never
//! does. `docs/concepts/watch.md`, *Ended not ours*.
//!
//! **Derived from the store's watch on every read**, so a restart keeps it; settling it keeps its
//! id among the dismissed, as for every item Fleet derives.

use ipc::{Instant, WaitingAct, WaitingActKind, WaitingItem, WaitingOption, WaitingSource};
use store::KeptWatchWhole;

use super::domain;
use super::telling::not_ours_said;

/// The id of the item for one watch: `watch_not_ours:<watch id>`.
pub(crate) const NOT_OURS_ITEM: &str = "watch_not_ours:";

/// Choice 0 settles it and tells nobody.
pub(crate) const IGNORE: u32 = 0;
/// Choice 1 asks the agent to fix what broke.
pub(crate) const FIX_IT: u32 = 1;

/// The item for `kept`, which ended `not_ours`: the watch and the parts that failed. Its act
/// names the watch. `None` where no set-aside part failed, so there is nothing to name.
pub(crate) fn not_ours_item(kept: &KeptWatchWhole) -> Option<WaitingItem> {
    let (parts, _) = not_ours_said(&domain(kept));
    if parts.is_empty() {
        return None;
    }
    let names: Vec<&str> = parts.iter().map(|(part, _)| part.as_str()).collect();
    let option = |label: &str| WaitingOption { label: label.to_string(), description: None };
    let held = &kept.watch;
    Some(WaitingItem {
        id: format!("{NOT_OURS_ITEM}{}", held.id),
        text: format!("{}: {} failed", held.name, names.join(", ")),
        since: Instant::carried(held.ended_at.as_deref().unwrap_or(&held.changed_at)),
        source: WaitingSource::WatchNotOurs,
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: held.id.clone() }),
        options: vec![option("Ignore"), option("Fix it")],
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
        confirmed: false,
    })
}

/// What the agent is sent on Fix it: each part, the sentence that set it aside, and the link.
pub(crate) fn fix_it(kept: &KeptWatchWhole) -> String {
    let (parts, link) = not_ours_said(&domain(kept));
    let names: Vec<&str> = parts.iter().map(|(part, _)| part.as_str()).collect();
    let mut said = format!(
        "Fix {} in {}. The person pressed Fix it on what you set aside as not ours.",
        names.join(", "),
        kept.watch.name
    );
    for (part, sentence) in &parts {
        said.push_str(&format!("\n{part}, set aside: {}", sentence.trim()));
    }
    if let Some(link) = link {
        said.push_str(&format!("\nThe failing run: {link}"));
    }
    said
}
