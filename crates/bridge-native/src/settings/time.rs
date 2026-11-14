//! Settings › Time (`TimeSettings`): how every stamp Bridge draws is written. Three switches, and a sample that is
//! the current moment in the format they make, so a change is read before it is left.

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, AnyElement, App, Context, FontWeight, KeyDownEvent, Role, SharedString, Window};
use ipc::time_format::TimeFormat;

use super::entries::Entry;
use super::overlay::SettingsOverlay;
use super::rows::{self, rows_card, split_row, Row, Spread};
use super::view::{ringed, switch_said};
use crate::labels::labelled;
use crate::look::{token, Look};

/// Each switch by its key in `time_format`, and its name.
pub const SWITCHES: [(&str, &str); 3] = [("hour12", "24-hour clock"), ("seconds", "Seconds"), ("date", "Date")];

thread_local! {
    /// The format Fleet last said he set (`timeFormat()`): read once a connection, and again after each save.
    static HELD: std::cell::Cell<TimeFormat> = std::cell::Cell::new(TimeFormat::default());
}

/// What he has set, for any stamp drawn on this thread.
pub fn held() -> TimeFormat {
    HELD.with(std::cell::Cell::get)
}

/// Take Fleet's `time_format` text, on a read or after a save. Text that is not a format is what ships.
pub fn hold(text: &str) {
    HELD.with(|held| held.set(ipc::time_format::read(text)));
}

/// The instant as he set it to be read (`stampOf`): "14:02:09", or "Nov 12 2:02 PM". With Date off, a day before
/// today's still shows its day, so a bare time is always today's.
pub fn stamp_of(at: DateTime<Local>, format: TimeFormat, now: DateTime<Local>) -> String {
    let time = time_of(at, format);
    if format.date || at.date_naive() != now.date_naive() { format!("{} {time}", at.format("%b %-d")) } else { time }
}

/// The clock alone, as the switches write it: for a time whose day the words around it already say.
pub fn time_of(at: DateTime<Local>, format: TimeFormat) -> String {
    match (format.hour12, format.seconds) {
        (true, true) => at.format("%-I:%M:%S %p"),
        (true, false) => at.format("%-I:%M %p"),
        (false, true) => at.format("%H:%M:%S"),
        (false, false) => at.format("%H:%M"),
    }
    .to_string()
}

/// A stamp in what he has set, read today.
pub fn stamp(at: DateTime<Local>) -> String {
    stamp_of(at, held(), Local::now())
}

/// A wire instant as a stamp in what he has set; `None` where it is no instant.
pub fn stamp_at(at: &str) -> Option<String> {
    DateTime::parse_from_rfc3339(at).ok().map(|at| stamp(at.with_timezone(&Local)))
}

/// The clock alone in what he has set.
pub fn clock(at: DateTime<Local>) -> String {
    time_of(at, held())
}

impl SettingsOverlay {
    fn time_format(&self) -> TimeFormat {
        ipc::time_format::read(&self.prefs.held().map(|held| held.time_format.clone()).unwrap_or_default())
    }

    /// A switch moved: the three are sent together, as Classic sends them.
    fn time_switch(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let mut format = self.time_format();
        match name {
            "hour12" => format.hour12 = !format.hour12,
            "seconds" => format.seconds = !format.seconds,
            "date" => format.date = !format.date,
            _ => return,
        }
        self.save_pref("time_format", format.text(), window, cx);
    }

    pub(super) fn time_stops(&self, _cx: &App) -> Vec<gpui::FocusHandle> {
        SWITCHES.iter().filter_map(|(name, _)| self.prefs.keys.get(&format!("time:{name}")).cloned()).collect()
    }

    pub(super) fn render_time(&self, entry: &'static Entry, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let spread = self.spread(look, window, cx);
        let card = match &self.prefs.read {
            None => self.row_bones(look, spread),
            Some(Err(said)) => labelled(super::view::refusal(look, said.clone()).id("time-unread"), Role::Alert, said.clone()).into_any_element(),
            Some(Ok(_)) => self.time_rows(spread, look, window, cx),
        };
        self.rows_page(entry, rows::purpose_of(entry).into(), false, Vec::new(), vec![card], look, window, cx)
    }

    /// What each switch writes, said once under its name.
    fn time_says(name: &str) -> &'static str {
        match name {
            "hour12" => "Off writes the 12-hour clock, with AM and PM.",
            "seconds" => "Whether a stamp shows its seconds.",
            _ => "Whether every stamp shows its day. A stamp from an earlier day always does.",
        }
    }

    fn time_rows(&self, spread: Spread, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let format = self.time_format();
        let off = !self.live;
        let mut rows: Vec<AnyElement> = SWITCHES
            .iter()
            .enumerate()
            .map(|(n, (name, said))| {
                let (on, words) = match *name {
                    "hour12" => (!format.hour12, ("24H", "12H")),
                    "seconds" => (format.seconds, ("ON", "OFF")),
                    _ => (format.date, ("ON", "OFF")),
                };
                let name: &'static str = name;
                let focus = self.prefs.keys.get(&format!("time:{name}")).cloned();
                let control = div()
                    .id(SharedString::from(format!("time-{name}")))
                    .flex_none()
                    .rounded(look.length(token::RADIUS_MD))
                    .child(switch_said(look, on, off, words))
                    .when_some(focus, |row, focus| ringed(look, row.track_focus(&focus), &focus, window))
                    .when(!off, |row| {
                        row.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| this.time_switch(name, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.time_switch(name, window, cx);
                            }
                        }))
                    });
                let control = div().flex().child(labelled(control, Role::Switch, *said));
                let row = Row {
                    id: SharedString::from(format!("row-time-{name}")),
                    title: (*said).into(),
                    tip: None,
                    chips: Vec::new(),
                    says: vec![rows::says(look, Self::time_says(name)).into_any_element()],
                    changed: false,
                    control: control.into_any_element(),
                };
                split_row(look, spread, row, n == 0).into_any_element()
            })
            .collect();
        let now = Local::now();
        let stamp = stamp_of(now, format, now);
        let sample = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .child(labelled(
                div().id("time-sample").text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(stamp)),
                Role::Label,
                "Sample",
            ))
            .children(self.prefs.refused.get("time_format").map(|said| labelled(super::view::refusal(look, said.clone()).id("time-refused"), Role::Alert, said.clone()).into_any_element()));
        rows.push(split_row(
            look,
            spread,
            Row {
                id: "row-time-sample".into(),
                title: "Sample".into(),
                tip: None,
                chips: Vec::new(),
                says: vec![rows::says(look, "The current moment, as the switches write it.").into_any_element()],
                changed: false,
                control: sample.into_any_element(),
            },
            false,
        ).into_any_element());
        rows_card(look, "settings-card-time", rows).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn a_held_format_is_what_fleet_last_said() {
        assert_eq!(held(), TimeFormat::default());
        hold(r#"{"hour12":true,"seconds":false,"date":false}"#);
        assert_eq!(held(), TimeFormat { hour12: true, seconds: false, date: false });
        hold("");
        assert_eq!(held(), TimeFormat::default());
    }

    #[test]
    fn each_switch_adds_to_the_stamp_and_an_earlier_day_keeps_its_day() {
        let now = Local.with_ymd_and_hms(2026, 11, 12, 14, 2, 9).single().expect("a time");
        let set = |hour12, seconds, date| TimeFormat { hour12, seconds, date };
        assert_eq!(stamp_of(now, TimeFormat::default(), now), "14:02:09");
        assert_eq!(stamp_of(now, set(true, false, false), now), "2:02 PM");
        assert_eq!(stamp_of(now, set(false, false, true), now), "Nov 12 14:02");
        let yesterday = Local.with_ymd_and_hms(2026, 11, 11, 9, 5, 0).single().expect("a time");
        assert_eq!(stamp_of(yesterday, set(false, false, false), now), "Nov 11 09:05");
    }
}
