//! The Mac a Needs-you command is meant for (`NeedsYouRow.on`), and whether Run is offered for it here.
//!
//! **Only the item's act says it**: the command's words are never read for a Mac. A command meant for another Mac
//! has no Run, so it is never run on the Mac he happens to sit at.

use ipc::Mac;

/// The Mac a command is meant for, where that is not this one: its name as listed, or as the item gave it. `None`
/// where the item names no Mac or names this one, by id or by name, `.local` or not. **A named Mac this window
/// cannot place is taken as another**, so a Gateway that does not answer leaves Run off rather than running another
/// Mac's command here.
pub(super) fn elsewhere(macs: &[Mac], on: Option<&str>) -> Option<String> {
    let on = on.map(str::trim).filter(|on| !on.is_empty())?;
    let bare = |name: &str| {
        let name = name.trim().to_ascii_lowercase();
        name.strip_suffix(".local").map_or_else(|| name.clone(), str::to_string)
    };
    match macs.iter().find(|one| one.id == on || bare(&one.name) == bare(on)) {
        Some(one) if one.this_mac => None,
        Some(one) => Some(one.name.clone()),
        None => Some(on.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::{MacColour, MacLink};

    fn this() -> Mac {
        Mac { id: "studio".into(), name: "Studio".into(), colour: MacColour::Teal, this_mac: true, ..Default::default() }
    }

    #[test]
    fn a_command_for_another_mac_is_elsewhere_and_one_for_this_mac_or_none_is_not() {
        let kiavi = Mac { id: "kiavi".into(), name: "Nick-Meles-MacBook-Pro---Kiavi".into(), colour: MacColour::Orange, link: Some(MacLink::Linked), ..Default::default() };
        let macs = [this(), kiavi];
        assert_eq!(elsewhere(&macs, None), None, "no Mac named: this one");
        assert_eq!(elsewhere(&macs, Some("  ")), None);
        assert_eq!(elsewhere(&macs, Some("studio.local")), None, "this Mac by its name");
        assert_eq!(elsewhere(&macs, Some("studio")), None, "this Mac by its id");
        assert_eq!(elsewhere(&macs, Some("nick-meles-macbook-pro---kiavi")).as_deref(), Some("Nick-Meles-MacBook-Pro---Kiavi"));
        assert_eq!(elsewhere(&macs, Some("Mini")).as_deref(), Some("Mini"), "a Mac the list lacks is another");
        assert_eq!(elsewhere(&[], Some("Studio")).as_deref(), Some("Studio"), "no list: never taken as this Mac");
    }
}
