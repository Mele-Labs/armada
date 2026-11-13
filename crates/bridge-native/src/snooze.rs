//! Snooze a Job: it keeps working and raises nothing to him until a time (`snooze_session`,
//! `docs/concepts/session/snoozing-a-job.md`). Fleet takes the instant; the presets are worked out here, in
//! local time, by [`instant`].
//!
//! Every door asks through [`crate::dispatch::screen::snooze_ask`]: the Jobs list's menu, the Job head's More
//! menu, the palette's panel and `z`. The answer is [`Snooze`], which the dispatch screen draws over the
//! window: a menu of the four presets, and for Custom a sheet with a date and a time. A wake needs no
//! choice, so each door posts it at once through [`crate::jobs::JobsList::snooze`].

use chrono::{DateTime, Datelike, Duration, LocalResult, Local, NaiveDate, NaiveDateTime, NaiveTime, SecondsFormat, TimeZone, Utc};
use gpui::{div, point, prelude::*, Anchor, App, Context, DismissEvent, Entity, FocusHandle, Focusable, FontWeight, KeyDownEvent, Pixels, Point, Role, SharedString, Subscription, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::time_format::TimeFormat;

use crate::dropdown::{Dropdown, Entry, Picked, Row};
use crate::jobs::JobsList;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{glyph, kbd, sans, Btn, Weight};

/// One of the choices the menu offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Hour,
    ThreeHours,
    /// Nine o'clock the next morning.
    Tomorrow,
    /// A time he picks on the sheet.
    Custom,
}

/// The presets in the menu's order, each by its row id and its words.
pub const PRESETS: [(Preset, &str, &str); 4] =
    [(Preset::Hour, "1h", "1 hour"), (Preset::ThreeHours, "3h", "3 hours"), (Preset::Tomorrow, "tomorrow", "Tomorrow"), (Preset::Custom, "custom", "Custom\u{2026}")];

/// Where a preset ends, counted from `now`; `None` for Custom, which he picks.
pub fn instant<Tz: TimeZone>(preset: Preset, now: &DateTime<Tz>) -> Option<DateTime<Tz>> {
    match preset {
        Preset::Hour => Some(now.clone() + Duration::hours(1)),
        Preset::ThreeHours => Some(now.clone() + Duration::hours(3)),
        Preset::Tomorrow => Some(tomorrow_at_nine(now)),
        Preset::Custom => None,
    }
}

/// Nine o'clock on the day after `now`'s, whatever the hour is now, and across a month's or a year's end.
pub fn tomorrow_at_nine<Tz: TimeZone>(now: &DateTime<Tz>) -> DateTime<Tz> {
    let today = now.date_naive();
    let day = today.succ_opt().unwrap_or(today);
    on(&now.timezone(), day.and_time(NaiveTime::from_hms_opt(9, 0, 0).unwrap_or(NaiveTime::MIN)))
}

/// A wall-clock time in `zone`. One the clocks repeat is its first; one they skip is the hour after.
fn on<Tz: TimeZone>(zone: &Tz, wall: NaiveDateTime) -> DateTime<Tz> {
    match zone.from_local_datetime(&wall) {
        LocalResult::Single(at) => at,
        LocalResult::Ambiguous(first, _) => first,
        LocalResult::None => zone.from_local_datetime(&(wall + Duration::hours(1))).earliest().unwrap_or_else(|| zone.from_utc_datetime(&wall)),
    }
}

const MONTHS: [&str; 12] = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];

/// A date as typed: `2026-11-14`, `Nov 14` or `14 Nov`. Without a year it is the next such day from `today`.
pub fn read_date(text: &str, today: NaiveDate) -> Option<NaiveDate> {
    let text = text.trim();
    if let Ok(date) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        return Some(date);
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let [one, two] = words[..] else { return None };
    let (day, month) = if one.chars().all(|c| c.is_ascii_digit()) { (one, two) } else { (two, one) };
    let day: u32 = day.parse().ok()?;
    let month = month.to_lowercase();
    let month = MONTHS.iter().position(|name| month.len() >= 3 && month.starts_with(name))? as u32 + 1;
    let this = NaiveDate::from_ymd_opt(today.year(), month, day)?;
    if this >= today { Some(this) } else { NaiveDate::from_ymd_opt(today.year() + 1, month, day) }
}

/// A time as typed: `9:00`, `09:00`, `21:30`, `9:00 pm`, `9pm` or `9`.
pub fn read_time(text: &str) -> Option<NaiveTime> {
    let text = text.trim().to_lowercase();
    let (clock, period) = match text.strip_suffix("am").map(|rest| (rest, Some(false))).or_else(|| text.strip_suffix("pm").map(|rest| (rest, Some(true)))) {
        Some((rest, pm)) => (rest.trim(), pm),
        None => (text.as_str(), None),
    };
    let (hour, minute) = clock.split_once(':').map_or((clock, "0"), |(hour, minute)| (hour, minute));
    let hour: u32 = hour.parse().ok()?;
    let minute: u32 = minute.parse().ok()?;
    let hour = match period {
        Some(pm) if (1..=12).contains(&hour) => hour % 12 + if pm { 12 } else { 0 },
        Some(_) => return None,
        None => hour,
    };
    NaiveTime::from_hms_opt(hour, minute, 0)
}

/// What the sheet's two fields make: the instant, or what is wrong with them.
pub fn read_picked<Tz: TimeZone>(date: &str, time: &str, now: &DateTime<Tz>) -> Result<DateTime<Tz>, &'static str> {
    let date = read_date(date, now.date_naive()).ok_or("Not a date")?;
    let time = read_time(time).ok_or("Not a time")?;
    let at = on(&now.timezone(), date.and_time(time));
    if at <= *now { Err("That time has passed") } else { Ok(at) }
}

/// A snooze's end as Fleet reads it.
pub fn instant_of(at: DateTime<Utc>) -> ipc::Instant {
    ipc::Instant::carried(at.to_rfc3339_opts(SecondsFormat::Millis, true))
}

/// The clock a snooze is counted from. A walk of the snoozed Jobs scenario keeps its own, so the times it
/// names read the same on every run.
pub fn now() -> DateTime<Local> {
    crate::mock::fixed_now().unwrap_or_else(Local::now)
}

/// A snooze's end as he has Bridge write a time. Seconds are left out, since a snooze has none; the day comes
/// with it where it is not today's, as every stamp in Bridge does.
pub fn when(at: DateTime<Local>, now: DateTime<Local>, format: TimeFormat) -> String {
    crate::settings::time::stamp_of(at, TimeFormat { seconds: false, ..format }, now)
}

/// The tooltip of a snoozed Job's quiet glyph.
pub fn quiet_until(at: DateTime<Local>, now: DateTime<Local>, format: TimeFormat) -> String {
    format!("Quiet until {}", when(at, now, format))
}

/// The wire's instant as a local one; `None` where it is no time.
pub fn local_of(at: &ipc::Instant) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(at.as_str()).ok().map(|at| at.with_timezone(&Local))
}

/// The act a Job's menu offers: Snooze, or Wake now on a Job that is snoozed. Both answer `Z`.
pub fn menu_item(snoozed: bool) -> Entry {
    if snoozed {
        Entry::Item { id: "wake", label: "Wake now".into(), icon: Some("bell"), danger: false, key: Some("Z".into()), off: None }
    } else {
        Entry::Item { id: "snooze", label: "Snooze".into(), icon: Some("bell-off"), danger: false, key: Some("Z".into()), off: None }
    }
}

/// The menu of presets, each saying when it ends.
pub fn preset_rows(now: DateTime<Local>, format: TimeFormat) -> Vec<Entry> {
    PRESETS
        .iter()
        .map(|(preset, id, label)| {
            let row = Row::new(id, *label);
            Entry::Row(match instant(*preset, &now) {
                Some(at) => row.detail(when(at, now, format)),
                None => row,
            })
        })
        .collect()
}

/// A snooze or a wake sent to Fleet. A refusal is said in a toast with Fleet's reason.
pub fn post(port: Option<u16>, body: Result<String, ipc::Unencodable>, cx: &mut App) {
    let Some(port) = port else {
        crate::toasts::tell("Fleet is not connected", cx);
        return;
    };
    let body = match body {
        Ok(body) => body,
        Err(why) => {
            crate::toasts::tell(format!("{why:?}"), cx);
            return;
        }
    };
    cx.spawn(async move |cx| {
        let answer = cx
            .background_executor()
            .spawn(async move {
                let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                runtime.block_on(crate::transport::post(port, "/sessions/snooze", body))
            })
            .await;
        let said = match answer {
            Ok((status, _)) if (200..300).contains(&status) => return,
            Ok((status, bytes)) => ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message),
            Err(why) => why,
        };
        cx.update(|cx| crate::toasts::tell(said, cx));
    })
    .detach();
}

/// What is open over the window: the presets' menu, or the sheet for Custom.
pub struct Snooze {
    list: Entity<JobsList>,
    menu: Option<OpenMenu>,
    sheet: Option<Sheet>,
}

struct OpenMenu {
    menu: Entity<Dropdown>,
    _subs: Vec<Subscription>,
}

struct Sheet {
    job: SharedString,
    title: SharedString,
    date: Entity<InputState>,
    time: Entity<InputState>,
    refused: Option<SharedString>,
    frame: FocusHandle,
    back: Option<FocusHandle>,
    _intercept: Subscription,
    _subs: Vec<Subscription>,
}

impl Snooze {
    pub fn new(list: Entity<JobsList>) -> Self {
        Snooze { list, menu: None, sheet: None }
    }

    /// The presets' menu at `at`, for the Job `job`, or at the top of the window where no door knew a place. The
    /// keys go back to where they were when it shuts.
    pub fn ask(&mut self, job: SharedString, title: SharedString, at: Option<Point<Pixels>>, window: &mut Window, cx: &mut Context<Self>) {
        self.shut(window, cx);
        // After the door that asked has put the keys back, so they return to the list or the Job and not to its menu.
        let back = window.focused(cx);
        let at = at.unwrap_or_else(|| {
            let view = crate::room::of(window, cx);
            point(view.origin.x + view.size.width / 2. - cx.look().length(token::W_MENU) / 2., view.origin.y + view.size.height * 0.2)
        });
        let entries = preset_rows(now(), crate::settings::time::held());
        let menu = cx.new(|cx| Dropdown::new("Snooze", entries, at, Anchor::TopLeft, true, cx));
        let (picked_job, picked_title, picked_back) = (job, title, back.clone());
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            this.chosen(id, picked_job.clone(), picked_title.clone(), picked_back.clone(), window, cx);
        });
        let dismissed = cx.subscribe_in(&menu, window, move |this, _, _: &DismissEvent, window, cx| {
            this.menu = None;
            if this.sheet.is_none() {
                if let Some(back) = &back {
                    window.focus(back, cx);
                }
            }
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.menu = Some(OpenMenu { menu, _subs: vec![picked, dismissed] });
        cx.notify();
    }

    fn chosen(&mut self, id: &str, job: SharedString, title: SharedString, back: Option<FocusHandle>, window: &mut Window, cx: &mut Context<Self>) {
        let Some((preset, ..)) = PRESETS.iter().find(|(_, held, _)| *held == id) else { return };
        match instant(*preset, &now()) {
            Some(at) => self.set(&job, at.with_timezone(&Utc), cx),
            None => self.open_sheet(job, title, back, window, cx),
        }
    }

    fn set(&mut self, job: &str, until: DateTime<Utc>, cx: &mut Context<Self>) {
        self.list.update(cx, |list, cx| list.snooze(job, Some(until), cx));
    }

    /// Custom: a date and a time, the date first, set to tomorrow at nine.
    fn open_sheet(&mut self, job: SharedString, title: SharedString, back: Option<FocusHandle>, window: &mut Window, cx: &mut Context<Self>) {
        let format = TimeFormat { seconds: false, ..crate::settings::time::held() };
        let first = tomorrow_at_nine(&now());
        let date = cx.new(|cx| InputState::new(window, cx).placeholder("2026-11-14"));
        let time = cx.new(|cx| InputState::new(window, cx).placeholder("9:00"));
        let (day, clock) = (first.format("%Y-%m-%d").to_string(), crate::settings::time::time_of(first, format));
        date.update(cx, |input, cx| input.set_value(day, window, cx));
        time.update(cx, |input, cx| input.set_value(clock, window, cx));
        let me = cx.entity().downgrade();
        let intercept = cx.intercept_keystrokes(move |event, window, cx| {
            let Some(me) = me.upgrade() else { return };
            me.update(cx, |this, cx| {
                if this.sheet.as_ref().is_some_and(|sheet| sheet.frame.contains_focused(window, cx)) {
                    let down = KeyDownEvent { keystroke: event.keystroke.clone(), is_held: false, prefer_character_input: false };
                    this.intercepted(&down, window, cx);
                }
            });
        });
        let entered = |input: &Entity<InputState>, cx: &mut Context<Self>, window: &mut Window| {
            cx.subscribe_in(input, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.submit(window, cx);
                }
            })
        };
        let subs = vec![entered(&date, cx, window), entered(&time, cx, window)];
        let first_box = date.clone();
        self.sheet = Some(Sheet { job, title, date, time, refused: None, frame: cx.focus_handle(), back, _intercept: intercept, _subs: subs });
        window.defer(cx, move |window, cx| first_box.update(cx, |input, cx| input.focus(window, cx)));
        cx.notify();
    }

    /// What the sheet takes before a text box can: Esc cancels, and Tab goes between the two boxes.
    fn intercepted(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(sheet) = self.sheet.as_ref() else { return };
        let key = &event.keystroke;
        let plain = !key.modifiers.platform && !key.modifiers.control && !key.modifiers.alt;
        match key.key.as_str() {
            "escape" => self.cancel(window, cx),
            "tab" if plain => {
                let (date, time) = (sheet.date.clone(), sheet.time.clone());
                let to = if date.focus_handle(cx).is_focused(window) { time } else { date };
                to.update(cx, |input, cx| input.focus(window, cx));
            }
            _ => {}
        }
    }

    /// Enter or Snooze: the two boxes are read, and a reading that fails is said under them.
    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(sheet) = self.sheet.as_mut() else { return };
        let (date, time) = (sheet.date.read(cx).value().to_string(), sheet.time.read(cx).value().to_string());
        match read_picked(&date, &time, &now()) {
            Ok(at) => {
                let job = sheet.job.clone();
                self.close_sheet(window, cx);
                self.set(&job, at.with_timezone(&Utc), cx);
            }
            Err(why) => {
                sheet.refused = Some(why.into());
                cx.notify();
            }
        }
    }

    /// Esc or Cancel: nothing is changed and the keys go back.
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_sheet(window, cx);
    }

    fn close_sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(back) = self.sheet.take().and_then(|sheet| sheet.back) {
            window.focus(&back, cx);
        }
        cx.notify();
    }

    fn shut(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        self.close_sheet(window, cx);
    }
}

/// A caption over a field, in sentence case.
fn caption(words: &'static str, look: &Look) -> gpui::Div {
    div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_MUTED)).child(words)
}

/// A text field's frame: sunken, one hairline, the accent and its ring while it has the keys.
fn field(look: &Look, focused: bool) -> gpui::Div {
    let accent = look.color(token::ACCENT);
    div()
        .h(look.length(token::H_CONTROL_LG))
        .px(look.length(token::SPACE_3))
        .flex()
        .items_center()
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(if focused { accent } else { look.color(token::BORDER_DEFAULT) })
        .when(focused, |field| {
            field.shadow(vec![gpui::BoxShadow { color: accent, offset: point(gpui::px(0.), gpui::px(0.)), blur_radius: gpui::px(0.), spread_radius: look.length(token::BORDER_WIDTH), inset: false }])
        })
        .bg(look.color(token::BG_SUNKEN))
        .text_size(look.length(token::TEXT_SM))
}

impl Render for Snooze {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(open) = &self.menu {
            return div().child(open.menu.clone()).into_any_element();
        }
        let Some(sheet) = &self.sheet else { return div().into_any_element() };
        let look = cx.look();
        let view = crate::room::of(window, cx).size;
        let width = look.length(token::W_DIALOG).min(view.width - look.length(token::SPACE_6) * 2.);
        let top = (view.height * 0.06).max(look.length(token::H_TITLE_BAR));
        let box_of = |input: &Entity<InputState>, name: &'static str, id: &'static str| {
            let focused = input.focus_handle(cx).is_focused(window);
            let text = div().flex_1().min_w_0().key_context(crate::keymap::TEXT_BOX).child(Input::new(input).appearance(false));
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(caption(name, look))
                .child(labelled(field(look, focused).id(id).child(text), Role::TextInput, name))
        };
        let head = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .child(glyph("bell-off", look.length(token::SPACE_4), look.color(token::FG_DEFAULT)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child(format!("Snooze {}", sheet.title)),
            )
            .child(kbd("Esc", look));
        let refusal = sheet.refused.clone().map(|why| labelled(div().id("snooze-refused").text_color(look.color(token::ERROR)).child(why.clone()), Role::Alert, why));
        let cancel = Btn::new("snooze-cancel", "Cancel", Weight::Secondary).draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx)));
        let set = Btn::new("snooze-set", "Snooze", Weight::Primary)
            .draw(look, cx)
            .shadow(look.shadows(token::SHADOW_PRIMARY))
            .on_click(cx.listener(|this, _, window, cx| this.submit(window, cx)))
            .child(kbd("\u{21a9}", look));
        let foot = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).children(refusal))
            .child(labelled(cancel, Role::Button, "Cancel"))
            .child(labelled(set, Role::Button, "Snooze"));
        let frame = div()
            .id("snooze-frame")
            .key_context("snooze")
            .track_focus(&sheet.frame)
            // A press on the sheet is not a press on the scrim.
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .w(width)
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_color(look.color(token::FG_DEFAULT))
            .font_family(sans(look))
            .child(head)
            .child(
                div()
                    .p(look.length(token::SPACE_4))
                    .flex()
                    .gap(look.length(token::SPACE_3))
                    .child(box_of(&sheet.date, "Date", "snooze-date"))
                    .child(box_of(&sheet.time, "Time", "snooze-time")),
            )
            .child(foot);
        let scrim = div()
            .id("snooze-scrim")
            .absolute()
            .inset_0()
            .occlude()
            .bg(look.color(token::SCRIM))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| this.cancel(window, cx)));
        let column = div().absolute().top(top).left((view.width - width) / 2.).child(frame);
        let name: SharedString = format!("Snooze {}", sheet.title).into();
        labelled(div().id("snooze").absolute().inset_0().child(scrim).child(column), Role::Dialog, name).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn at(zone: i32, y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<FixedOffset> {
        FixedOffset::east_opt(zone * 3600).expect("a zone").with_ymd_and_hms(y, m, d, h, min, 0).single().expect("a time")
    }

    #[test]
    fn an_hour_and_three_hours_count_from_now() {
        let now = at(1, 2026, 11, 13, 14, 2);
        assert_eq!(instant(Preset::Hour, &now), Some(at(1, 2026, 11, 13, 15, 2)));
        assert_eq!(instant(Preset::ThreeHours, &now), Some(at(1, 2026, 11, 13, 17, 2)));
    }

    #[test]
    fn three_hours_late_in_the_evening_cross_midnight() {
        let now = at(0, 2026, 11, 13, 22, 30);
        assert_eq!(instant(Preset::ThreeHours, &now), Some(at(0, 2026, 11, 14, 1, 30)));
    }

    #[test]
    fn custom_is_left_for_him_to_pick() {
        assert_eq!(instant(Preset::Custom, &at(0, 2026, 11, 13, 14, 2)), None);
    }

    #[test]
    fn tomorrow_is_nine_the_next_morning_at_any_hour() {
        for hour in [0, 8, 9, 14, 23] {
            assert_eq!(instant(Preset::Tomorrow, &at(2, 2026, 11, 13, hour, 59)), Some(at(2, 2026, 11, 14, 9, 0)), "from {hour}:59");
        }
    }

    #[test]
    fn tomorrow_crosses_a_day_end_a_month_end_and_a_year_end() {
        assert_eq!(tomorrow_at_nine(&at(0, 2026, 11, 13, 23, 59)), at(0, 2026, 11, 14, 9, 0));
        assert_eq!(tomorrow_at_nine(&at(0, 2026, 11, 30, 10, 0)), at(0, 2026, 12, 1, 9, 0));
        assert_eq!(tomorrow_at_nine(&at(0, 2026, 12, 31, 18, 0)), at(0, 2027, 1, 1, 9, 0));
        assert_eq!(tomorrow_at_nine(&at(0, 2028, 2, 28, 18, 0)), at(0, 2028, 2, 29, 9, 0), "a leap year has its day");
        assert_eq!(tomorrow_at_nine(&at(0, 2027, 2, 28, 18, 0)), at(0, 2027, 3, 1, 9, 0));
    }

    #[test]
    fn tomorrow_is_counted_in_his_own_zone() {
        // 23:30 in UTC+9 is 14:30 UTC: tomorrow is the 14th there and nine in the morning of it.
        let now = at(9, 2026, 11, 13, 23, 30);
        let tomorrow = tomorrow_at_nine(&now);
        assert_eq!(tomorrow, at(9, 2026, 11, 14, 9, 0));
        assert_eq!(tomorrow.with_timezone(&Utc), Utc.with_ymd_and_hms(2026, 11, 14, 0, 0, 0).single().expect("a time"));
    }

    #[test]
    fn a_time_is_read_as_typed() {
        let t = |h, m| NaiveTime::from_hms_opt(h, m, 0);
        assert_eq!(read_time("9:00"), t(9, 0));
        assert_eq!(read_time("09:05"), t(9, 5));
        assert_eq!(read_time("21:30"), t(21, 30));
        assert_eq!(read_time("9"), t(9, 0));
        assert_eq!(read_time("9:00 pm"), t(21, 0));
        assert_eq!(read_time("9pm"), t(21, 0));
        assert_eq!(read_time("12:15 AM"), t(0, 15));
        assert_eq!(read_time("12 pm"), t(12, 0));
        assert_eq!(read_time("13 pm"), None);
        assert_eq!(read_time("25:00"), None);
        assert_eq!(read_time("9:75"), None);
        assert_eq!(read_time("soon"), None);
        assert_eq!(read_time(""), None);
    }

    #[test]
    fn a_date_is_read_as_typed_and_a_bare_day_is_the_next_one() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 13).expect("a date");
        let d = |y, m, d| NaiveDate::from_ymd_opt(y, m, d);
        assert_eq!(read_date("2026-11-14", today), d(2026, 11, 14));
        assert_eq!(read_date("Nov 14", today), d(2026, 11, 14));
        assert_eq!(read_date("14 nov", today), d(2026, 11, 14));
        assert_eq!(read_date("December 3", today), d(2026, 12, 3));
        assert_eq!(read_date("Nov 13", today), d(2026, 11, 13), "today is still today");
        assert_eq!(read_date("Jan 5", today), d(2027, 1, 5), "a day that has gone round is next year's");
        assert_eq!(read_date("Feb 30", today), None);
        assert_eq!(read_date("tomorrow", today), None);
        assert_eq!(read_date("2026-13-01", today), None);
    }

    #[test]
    fn the_sheet_reads_its_two_boxes_and_says_what_is_wrong() {
        let now = at(0, 2026, 11, 13, 14, 2);
        assert_eq!(read_picked("2026-11-14", "9:00", &now), Ok(at(0, 2026, 11, 14, 9, 0)));
        assert_eq!(read_picked("Nov 13", "5pm", &now), Ok(at(0, 2026, 11, 13, 17, 0)));
        assert_eq!(read_picked("later", "9:00", &now), Err("Not a date"));
        assert_eq!(read_picked("2026-11-14", "later", &now), Err("Not a time"));
        assert_eq!(read_picked("2026-11-13", "13:00", &now), Err("That time has passed"));
        assert_eq!(read_picked("2026-11-13", "14:02", &now), Err("That time has passed"), "this minute is not later");
    }

    #[test]
    fn the_wire_takes_an_instant_in_utc_to_the_millisecond() {
        let at = Utc.with_ymd_and_hms(2026, 11, 14, 9, 0, 0).single().expect("a time");
        assert_eq!(instant_of(at).as_str(), "2026-11-14T09:00:00.000Z");
        assert_eq!(local_of(&instant_of(at)).map(|local| local.with_timezone(&Utc)), Some(at));
        assert_eq!(local_of(&ipc::Instant::carried("soon")), None);
    }

    fn local(d: u32, h: u32, m: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, d, h, m, 0).single().expect("a local time")
    }

    #[test]
    fn the_tooltip_says_the_time_alone_today_and_the_day_with_it_after() {
        let now = local(13, 14, 2);
        let format = TimeFormat { hour12: false, seconds: true, date: false };
        assert_eq!(quiet_until(local(13, 16, 30), now, format), "Quiet until 16:30", "seconds are left out");
        assert_eq!(quiet_until(local(14, 9, 0), now, format), "Quiet until Nov 14 09:00");
        assert_eq!(quiet_until(local(13, 16, 30), now, TimeFormat { date: true, ..format }), "Quiet until Nov 13 16:30");
        assert_eq!(quiet_until(local(13, 16, 30), now, TimeFormat { hour12: true, ..format }), "Quiet until 4:30 PM");
    }

    #[test]
    fn the_menu_offers_wake_now_in_place_of_snooze_on_a_snoozed_job() {
        let label = |entry: Entry| match entry {
            Entry::Item { label, .. } => label.to_string(),
            _ => String::new(),
        };
        assert_eq!(label(menu_item(false)), "Snooze");
        assert_eq!(label(menu_item(true)), "Wake now");
    }

    #[test]
    fn the_presets_say_when_each_ends() {
        let rows = preset_rows(local(13, 14, 2), TimeFormat { hour12: false, seconds: false, date: false });
        let said: Vec<(String, Option<String>)> = rows
            .into_iter()
            .map(|entry| match entry {
                Entry::Row(row) => (row.label.to_string(), row.detail.map(|detail| detail.to_string())),
                _ => (String::new(), None),
            })
            .collect();
        assert_eq!(
            said,
            [
                ("1 hour".to_string(), Some("15:02".to_string())),
                ("3 hours".to_string(), Some("17:02".to_string())),
                ("Tomorrow".to_string(), Some("Nov 14 09:00".to_string())),
                ("Custom\u{2026}".to_string(), None),
            ]
        );
    }
}
