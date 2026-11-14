//! The Disk section of the Fleet panel: free space where Jobs are built, against the two floors Fleet holds
//! it to, and what it is doing about it. What the section says is worked out here from `GET /health`'s
//! `disk` and nothing else; the panel draws it.

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, relative, AnyElement, FontWeight, Hsla, Role, SharedString};
use ipc::DiskHealth;

use super::parts::{glyph, mono, small, tip};
use crate::labels::labelled;
use crate::look::{token, Look};

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

/// How many folders the section names: the wire carries ten, and a panel names the next few.
const NAMED: usize = 3;

/// How many characters of a folder's path the panel has room for, in the mono face at the section's size.
const ROOM: usize = 30;

/// Where the disk stands against the floors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Over the reclaim floor.
    Over,
    /// Under the reclaim floor: Fleet drops build output.
    Reclaiming,
    /// Under the hold floor: no new Job starts. Wins over the others.
    Held,
    /// Reclaim is off and the disk is over the hold floor.
    ReclaimOff,
}

pub fn state_of(disk: &DiskHealth) -> State {
    if disk.free_bytes < disk.hold_floor_bytes {
        State::Held
    } else if disk.reclaim_floor_bytes == 0 {
        State::ReclaimOff
    } else if disk.free_bytes < disk.reclaim_floor_bytes {
        State::Reclaiming
    } else {
        State::Over
    }
}

/// Bytes as GB (1 GB is 1024^3, as the floors are set): no decimals from 10, one under.
pub fn gb(bytes: u64) -> String {
    let gib = bytes as f64 / GIB;
    if (gib * 10.0).round() >= 100.0 { format!("{} GB", gib.round()) } else { format!("{:.1} GB", gib) }
}

/// The fact the state puts under the meter.
pub fn fact(disk: &DiskHealth) -> String {
    match state_of(disk) {
        State::Held => format!("No new Job starts under {}", gb(disk.hold_floor_bytes)),
        State::ReclaimOff => "Reclaim is off".to_string(),
        State::Reclaiming => format!("Dropping build output under {}", gb(disk.reclaim_floor_bytes)),
        State::Over => format!("Drops build output under {}", gb(disk.reclaim_floor_bytes)),
    }
}

/// What the meter is, said whole for a reader that cannot see it.
pub fn meter_said(disk: &DiskHealth) -> String {
    match disk.reclaim_floor_bytes {
        0 => format!("{} free, reclaim is off", gb(disk.free_bytes)),
        floor => format!("{} free of a {} floor", gb(disk.free_bytes), gb(floor)),
    }
}

/// What the meter's full width stands for: twice the reclaim floor, or with reclaim off four times the hold floor.
fn scale(disk: &DiskHealth) -> Option<u64> {
    match (disk.reclaim_floor_bytes, disk.hold_floor_bytes) {
        (0, 0) => None,
        (0, hold) => Some(hold.saturating_mul(4)),
        (reclaim, _) => Some(reclaim.saturating_mul(2)),
    }
}

/// How far along the meter `bytes` is, 0 to 1.
fn along(bytes: u64, whole: u64) -> f32 {
    (bytes as f64 / whole as f64).clamp(0.0, 1.0) as f32
}

/// A path cut in the middle to `room` characters, since what tells one build folder from the next is its end.
/// The mono face makes a character a width; the whole path stays in the tooltip.
pub fn elided(path: &str, room: usize) -> String {
    let chars: Vec<char> = path.chars().collect();
    if chars.len() <= room {
        return path.to_string();
    }
    let tail = room * 3 / 5;
    let head = room - tail - 1;
    format!("{}…{}", chars[..head].iter().collect::<String>(), chars[chars.len() - tail..].iter().collect::<String>())
}

/// "Freed 12 GB at 14:02:09", the time in the format he has set.
fn last_pass(disk: &DiskHealth, now: DateTime<Local>) -> Option<String> {
    let pass = disk.last_pass.as_ref()?;
    let at = DateTime::parse_from_rfc3339(&pass.at).ok()?.with_timezone(&Local);
    Some(format!("Freed {} at {}", gb(pass.freed_bytes), crate::settings::time::stamp_of(at, crate::settings::time::held(), now)))
}

fn tone(state: State, look: &Look) -> Hsla {
    look.color(match state {
        State::Over | State::ReclaimOff => token::STATUS_COMPLETED_SUCCESS,
        State::Reclaiming => token::STATUS_AWAITING_REVIEW,
        State::Held => token::STATUS_COMPLETED_FAILED,
    })
}

/// The hue the disk's state is drawn in, on the panel's Disk row as in the section.
pub fn ink(disk: &DiskHealth, look: &Look) -> Hsla {
    tone(state_of(disk), look)
}

/// How far along the meter the free space is, 0 to 1, for the Disk row's bar. None where no floor is set.
pub fn filled(disk: &DiskHealth) -> Option<f32> {
    scale(disk).map(|whole| along(disk.free_bytes, whole))
}

/// The section: the head, the meter, the fact, the last pass and the folders next to go.
pub fn section(disk: &DiskHealth, look: &Look) -> AnyElement {
    let state = state_of(disk);
    let ink = tone(state, look);
    let subtle = look.color(token::FG_SUBTLE);
    let head = div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .child(glyph("hard-drive", look.length(token::SPACE_4), ink))
        .child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child("Disk"))
        .child(div().flex_1())
        .child(div().font_family(mono(look)).text_color(ink).child(format!("{} free", gb(disk.free_bytes))));
    let track = look.length(token::SPACE_1);
    let tick = |at: f32, id: &'static str| {
        div()
            .id(id)
            .absolute()
            .top_0()
            .left(relative(at))
            .w(look.length(token::BORDER_WIDTH))
            .h(look.length(token::SPACE_2))
            .bg(look.color(token::FG_MUTED))
    };
    let meter = scale(disk).map(|whole| {
        let ticks = [(disk.hold_floor_bytes, "disk-hold-floor"), (disk.reclaim_floor_bytes, "disk-reclaim-floor")];
        labelled(
            div()
                .id("disk-meter")
                .relative()
                .h(look.length(token::SPACE_2))
                .flex()
                .items_center()
                .child(
                    div()
                        .w_full()
                        .h(track)
                        .rounded_full()
                        .overflow_hidden()
                        .bg(look.color(token::BG_SUNKEN))
                        .child(div().h_full().w(relative(along(disk.free_bytes, whole))).rounded_full().bg(ink)),
                )
                .children(ticks.into_iter().filter(|(floor, _)| *floor > 0).map(|(floor, id)| tick(along(floor, whole), id))),
            Role::Image,
            meter_said(disk),
        )
    });
    let line = |said: String| small(look).text_color(subtle).child(said);
    let named: Vec<&String> = disk.next_to_drop.iter().take(NAMED).collect();
    let next = (!named.is_empty()).then(|| {
        labelled(
            div()
                .id("disk-next")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(line("Next to drop".to_string()))
                .children(named.into_iter().enumerate().map(|(at, path)| {
                    let said: SharedString = path.clone().into();
                    labelled(
                        div()
                            .id(("disk-path", at))
                            .min_w_0()
                            .truncate()
                            .text_size(look.length(token::TEXT_2XS))
                            .line_height(look.length(token::LEADING_2XS))
                            .font_family(mono(look))
                            .text_color(look.color(token::FG_MUTED))
                            .tooltip(tip(said.clone()))
                            .child(elided(path, ROOM)),
                        Role::ListItem,
                        said,
                    )
                })),
            Role::List,
            "Next to drop",
        )
    });
    labelled(
        div()
            .id("disk")
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .child(head)
            .children(meter)
            .child(line(fact(disk)))
            .children(last_pass(disk, Local::now()).map(line))
            .children(next),
        Role::Group,
        "Disk",
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    const G: u64 = 1024 * 1024 * 1024;

    fn disk(free: u64, reclaim: u64, hold: u64) -> DiskHealth {
        DiskHealth { free_bytes: free * G, reclaim_floor_bytes: reclaim * G, hold_floor_bytes: hold * G, last_pass: None, next_to_drop: Vec::new() }
    }

    #[test]
    fn gb_has_no_decimals_from_ten_and_one_under() {
        assert_eq!(gb(142 * G), "142 GB");
        assert_eq!(gb(10 * G), "10 GB");
        assert_eq!(gb(G * 21 / 2), "11 GB");
        assert_eq!(gb(G * 19 / 2), "9.5 GB");
        assert_eq!(gb(0), "0.0 GB");
        assert_eq!(gb(G * 3 / 5), "0.6 GB");
        // 9.96 rounds up to ten, which reads as ten and not as 10.0.
        assert_eq!(gb((9.96 * GIB) as u64), "10 GB");
    }

    #[test]
    fn the_state_is_over_reclaiming_held_or_reclaim_off() {
        assert_eq!(state_of(&disk(142, 100, 30)), State::Over);
        assert_eq!(state_of(&disk(100, 100, 30)), State::Over);
        assert_eq!(state_of(&disk(64, 100, 30)), State::Reclaiming);
        assert_eq!(state_of(&disk(30, 100, 30)), State::Reclaiming);
        assert_eq!(state_of(&disk(22, 100, 30)), State::Held);
        assert_eq!(state_of(&disk(50, 0, 30)), State::ReclaimOff);
        // Held outranks the rest, reclaim off or not.
        assert_eq!(state_of(&disk(12, 0, 30)), State::Held);
    }

    #[test]
    fn each_state_says_its_own_fact_with_the_floor_it_names() {
        assert_eq!(fact(&disk(142, 100, 30)), "Drops build output under 100 GB");
        assert_eq!(fact(&disk(64, 100, 30)), "Dropping build output under 100 GB");
        assert_eq!(fact(&disk(22, 100, 30)), "No new Job starts under 30 GB");
        assert_eq!(fact(&disk(50, 0, 30)), "Reclaim is off");
    }

    #[test]
    fn a_long_path_is_cut_in_the_middle_and_keeps_its_end() {
        let path = "/Users/me/dev/app/.armada/slots/slot-4/target";
        assert_eq!(elided(path, 60), path);
        let cut = elided(path, 30);
        assert_eq!(cut.chars().count(), 30);
        assert!(cut.starts_with("/Users/me/d") && cut.ends_with("lots/slot-4/target"), "{cut}");
        assert!(cut.contains('…'));
    }

    #[test]
    fn the_meter_is_said_whole() {
        assert_eq!(meter_said(&disk(142, 100, 30)), "142 GB free of a 100 GB floor");
        assert_eq!(meter_said(&disk(50, 0, 30)), "50 GB free, reclaim is off");
    }

    #[test]
    fn the_meter_runs_to_twice_the_reclaim_floor_and_stops_at_full() {
        let over = disk(142, 100, 30);
        let whole = scale(&over).expect("a floor is set");
        assert_eq!(whole, 200 * G);
        assert_eq!(along(over.reclaim_floor_bytes, whole), 0.5);
        assert_eq!(along(over.free_bytes, whole), 0.71);
        assert_eq!(along(500 * G, whole), 1.0);
        assert_eq!(scale(&disk(50, 0, 30)), Some(120 * G));
        assert_eq!(scale(&disk(50, 0, 0)), None);
    }

    #[test]
    fn a_last_pass_reads_in_the_set_time_format_and_a_bad_instant_reads_nothing() {
        let now = Local::now();
        let mut held = disk(64, 100, 30);
        assert_eq!(last_pass(&held, now), None);
        held.last_pass = Some(ipc::ReclaimPass { at: now.to_rfc3339(), freed_bytes: 12 * G });
        let said = last_pass(&held, now).expect("a pass");
        assert!(said.starts_with("Freed 12 GB at "), "{said}");
        held.last_pass = Some(ipc::ReclaimPass { at: "not an instant".into(), freed_bytes: 12 * G });
        assert_eq!(last_pass(&held, now), None);
    }
}
