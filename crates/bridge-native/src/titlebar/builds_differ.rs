//! Two Macs left on different builds, as a held toast: a Refresh of both Macs that did not land the same commit on
//! each. The Settings page draws the build amber; this is what tells him without opening it.
//!
//! **It shows for as long as the builds differ and has stood so for [`STANDS_FOR`]**: the Gateway lists each Mac's
//! build every half minute, so a Mac that has just restarted onto the new build is still listed on the old one
//! for a while, and that is not a difference. It is never shown while a restart or a Refresh is under way, nor
//! while the other Mac is asleep, since a sleeping Mac's build is the one it last said.

use std::time::{Duration, Instant};

use ipc::{BuildSource, Mac, MacLink};

use crate::toasts::FleetNotice;

/// The id the toast is held under.
pub(super) const ID: &str = "builds-differ";
/// How long the builds must have differed before he is told. A little over the Gateway's own half minute.
pub(super) const STANDS_FOR: Duration = Duration::from_secs(45);

/// This Mac and the other, where both say a build and the builds differ.
fn differing(macs: &[Mac]) -> Option<(&Mac, &Mac)> {
    let this = macs.iter().find(|one| one.this_mac)?;
    let other = crate::pairing::refresh_partner(macs).filter(|one| one.link == Some(MacLink::Linked))?;
    match (this.build.as_deref(), other.build.as_deref()) {
        (Some(mine), Some(theirs)) if !mine.is_empty() && !theirs.is_empty() && mine != theirs => Some((this, other)),
        _ => None,
    }
}

/// A build's hash as the bar shows it.
fn short(build: &str) -> &str {
    build.get(..7).unwrap_or(build)
}

/// When the builds first differed, and the toast once they have for long enough.
#[derive(Default)]
pub(super) struct Differing {
    since: Option<Instant>,
}

impl Differing {
    /// Read the Macs as of `now`. `on` is the build this Mac runs and `restarting` whether a restart or a Refresh
    /// is under way; a Refresh is how a Mac comes to run the preview, and a Mac on main is a choice, not a miss.
    /// Returns the toast to hold, and how long until the answer would change where it is not yet held.
    pub(super) fn read(&mut self, macs: &[Mac], on: BuildSource, restarting: bool, now: Instant) -> (Option<FleetNotice>, Option<Duration>) {
        let found = if on == BuildSource::Preview && !restarting { differing(macs) } else { None };
        let Some((this, other)) = found else {
            self.since = None;
            return (None, None);
        };
        let since = *self.since.get_or_insert(now);
        let stood = now.saturating_duration_since(since);
        if stood < STANDS_FOR {
            return (None, Some(STANDS_FOR - stood));
        }
        let (mine, theirs) = (short(this.build.as_deref().unwrap_or_default()), short(other.build.as_deref().unwrap_or_default()));
        let notice = FleetNotice {
            id: ID,
            headline: format!("{} and {} are on different builds", this.name, other.name).into(),
            next: format!("{} runs {mine}, {} runs {theirs}. Refresh both Macs again.", this.name, other.name).into(),
            code: "builds-differ",
            fault: false,
            act: None,
            closable: true,
        };
        (Some(notice), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::MacColour;

    fn studio(build: &str) -> Mac {
        Mac { id: "studio".into(), name: "Studio".into(), colour: MacColour::Teal, this_mac: true, build: Some(build.into()), ..Default::default() }
    }

    fn air(build: &str, link: MacLink) -> Mac {
        Mac { id: "air".into(), name: "Air".into(), colour: MacColour::Orange, build: Some(build.into()), link: Some(link), ..Default::default() }
    }

    fn after(seconds: u64, start: Instant) -> Instant {
        start + Duration::from_secs(seconds)
    }

    #[test]
    fn builds_that_differ_after_a_refresh_are_said_once_they_have_stood_and_cleared_when_they_match() {
        let (start, mut watch) = (Instant::now(), Differing::default());
        let apart = [studio("2947ff9aaaaaaaa"), air("4e5fe24bbbbbbbb", MacLink::Linked)];
        let (held, wait) = watch.read(&apart, BuildSource::Preview, false, start);
        assert!(held.is_none(), "the Gateway may only be behind");
        assert_eq!(wait, Some(STANDS_FOR));
        let (held, wait) = watch.read(&apart, BuildSource::Preview, false, after(46, start));
        let held = held.expect("held once it has stood");
        assert_eq!(wait, None);
        assert_eq!(held.id, ID);
        assert_eq!(held.headline.as_ref(), "Studio and Air are on different builds");
        assert_eq!(held.next.as_ref(), "Studio runs 2947ff9, Air runs 4e5fe24. Refresh both Macs again.");
        let level = [studio("2947ff9aaaaaaaa"), air("2947ff9aaaaaaaa", MacLink::Linked)];
        assert_eq!(watch.read(&level, BuildSource::Preview, false, after(60, start)), (None, None), "cleared when they match");
        let (held, _) = watch.read(&apart, BuildSource::Preview, false, after(61, start));
        assert!(held.is_none(), "a difference that comes back stands again from the start");
    }

    #[test]
    fn nothing_is_said_during_a_restart_on_main_or_while_the_other_mac_sleeps() {
        let (start, mut watch) = (Instant::now(), Differing::default());
        let apart = [studio("2947ff9aaaaaaaa"), air("4e5fe24bbbbbbbb", MacLink::Linked)];
        let later = after(120, start);
        assert!(watch.read(&apart, BuildSource::Preview, true, start).0.is_none(), "a restart is under way");
        assert!(watch.read(&apart, BuildSource::Main, false, start).0.is_none(), "a Mac on main took no Refresh");
        let asleep = [studio("2947ff9aaaaaaaa"), air("4e5fe24bbbbbbbb", MacLink::Asleep { since: 0 })];
        assert!(watch.read(&asleep, BuildSource::Preview, false, later).0.is_none(), "its build is the one it last said");
        let unsaid = [studio("2947ff9aaaaaaaa"), Mac { build: None, ..air("", MacLink::Linked) }];
        assert!(watch.read(&unsaid, BuildSource::Preview, false, later).0.is_none(), "a Mac that does not say its build");
    }
}
