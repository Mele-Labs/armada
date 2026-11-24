//! A Mac whose Jobs cannot be reached, in the Jobs list: asleep, or frozen on a build that differs. Its Jobs leave their groups and stand under one fold at the
//! foot, shut until he opens it, dimmed and with nothing that pulls at him: no needs edge, no state glyph.
//! They stay findable, and open as any Job does.

use std::collections::BTreeSet;

use super::*;

/// Marks a group's key as one under a Mac's fold: this, the Mac's id, this again, then the group's own key.
const KEY: &str = "asleep\u{1}";

/// The fold of one Mac that is asleep.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Shelf {
    mac: SharedString,
    name: SharedString,
    colour: ipc::MacColour,
    why: crate::pairing::Why,
    /// The Mac's own words for a frozen link, the fold's tooltip in place of the press it offers.
    tip: Option<SharedString>,
    open: bool,
}

impl Shelf {
    /// The id the keys carry while they are on the fold, which no Job's id can be.
    pub(super) fn id(&self) -> SharedString {
        format!("asleep:{}", self.mac).into()
    }
}

/// The Jobs on Macs that are asleep as groups of their own, one for each Mac and project, in the order the Macs
/// are listed. Each group keeps its project's name, so a row still says which project it is in.
pub(super) fn groups<'a>(sleeping: &[&'a SessionRecord], asleep: &[crate::pairing::Unreachable], names: &Names) -> Vec<(String, String, Vec<&'a SessionRecord>)> {
    let mut out: Vec<(String, String, Vec<&'a SessionRecord>)> = Vec::new();
    for one in asleep {
        for job in sleeping.iter().filter(|job| job.mac.as_deref() == Some(one.id.as_str())) {
            let (key, name) = group_of(job, names);
            let key = format!("{KEY}{}{KEY}{key}", one.id);
            match out.iter_mut().find(|(held, ..)| *held == key) {
                Some((.., held)) => held.push(job),
                None => out.push((key, name, vec![job])),
            }
        }
    }
    out
}

/// Where a group under a Mac's fold begins: the fold itself, once for the Mac, in place of a project's name. False for any other group.
pub(super) fn open_line(key: &str, asleep: &[crate::pairing::Unreachable], lines: &mut Vec<Line>) -> bool {
    let Some(rest) = key.strip_prefix(KEY) else { return false };
    let id = rest.split(KEY).next().unwrap_or_default();
    let stands = lines.iter().any(|line| matches!(line, Line::Asleep(shelf) if shelf.mac == id));
    if let (false, Some(one)) = (stands, asleep.iter().find(|one| one.id == id)) {
        lines.push(Line::Asleep(Shelf { mac: one.id.clone().into(), name: one.name.clone().into(), colour: one.colour, why: one.why, tip: one.tip.clone().map(Into::into), open: true }));
    }
    true
}

/// `lines` with each fold shut, its Jobs out of sight, unless its Mac is in `open`.
pub(super) fn shelve_open(lines: Vec<Line>, open: &BTreeSet<String>) -> Vec<Line> {
    let mut hidden = false;
    lines
        .into_iter()
        .filter_map(|line| match line {
            Line::Asleep(mut shelf) => {
                shelf.open = open.contains(shelf.mac.as_ref());
                hidden = !shelf.open;
                Some(Line::Asleep(shelf))
            }
            Line::Job(_) if hidden => None,
            other => Some(other),
        })
        .collect()
}

impl JobsList {
    /// The window's Macs moved: the Jobs of one that is asleep go under its fold, and a Mac awake again lets go of its fold's state.
    pub(super) fn follow_macs(&mut self, cx: &mut Context<Self>) {
        let asleep = crate::pairing::unreachable_of(crate::pairing::macs(cx));
        if asleep == self.asleep {
            return;
        }
        self.asleep = asleep;
        self.asleep_open.retain(|id| self.asleep.iter().any(|one| &one.id == id));
        self.relist(cx);
    }

    /// Open or shut the fold of a Mac asleep. Shut, the keys that were on one of its Jobs go to the fold.
    pub(super) fn set_shelf(&mut self, mac: &str, open: bool, cx: &mut Context<Self>) {
        if open == self.asleep_open.contains(mac) {
            return;
        }
        if open {
            self.asleep_open.insert(mac.to_string());
        } else {
            self.asleep_open.remove(mac);
            if self.focused.as_ref().is_some_and(|id| self.rows().any(|row| &row.id == id && row.asleep && row.mac.as_deref() == Some(mac))) {
                self.focused = Some(format!("asleep:{mac}").into());
            }
        }
        self.relist(cx);
    }

    /// The Mac whose fold the keys are on, and whether it is open.
    pub(super) fn focused_shelf(&self) -> Option<(SharedString, bool)> {
        let id = self.focused.as_ref()?;
        self.lines.iter().find_map(|line| match line {
            Line::Asleep(shelf) if shelf.id() == *id => Some((shelf.mac.clone(), shelf.open)),
            _ => None,
        })
    }

    /// The fold of a Mac that is asleep: a chevron, the Mac's dot, its name, and since when it sleeps at the right.
    /// A press, ↵ or Space opens it, and its Jobs stand under it, dimmed.
    pub(super) fn render_shelf(&self, at: usize, shelf: &Shelf, cx: &mut Context<Self>) -> gpui::AnyElement {
        let look = cx.look();
        let focused = self.focused.as_ref() == Some(&shelf.id());
        let after_shelf = at > 0 && matches!(self.lines[at - 1], Line::Asleep(_));
        let said = shelf.why.said();
        let chip = crate::pairing::Chip { name: shelf.name.to_string(), colour: shelf.colour, asleep_since: None };
        let mark_glyph = look.length(token::SPACE_3);
        let tip: SharedString = shelf.tip.clone().unwrap_or_else(|| if shelf.open { format!("Hide the Jobs on {}", shelf.name) } else { format!("Show the Jobs on {}", shelf.name) }.into());
        let (mac, id) = (shelf.mac.clone(), shelf.id());
        let open = shelf.open;
        let name: SharedString = format!("{}, {said}", shelf.name).into();
        let strip = row_card(look, SharedString::from(format!("asleep-{}", shelf.mac)), false, focused, true, look.length(token::SPACE_2))
            .cursor_pointer()
            .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(tip.clone()).build(window, cx))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.focused = Some(id.clone());
                window.focus(&this.focus, cx);
                this.set_shelf(&mac, !open, cx);
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::H_CONTROL_SM))
                    .child(
                        div()
                            .flex_none()
                            .size(look.length(token::SPACE_4))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(svg().path(icon_path(if open { "chevron-down" } else { "chevron-right" })).size(mark_glyph).text_color(look.color(token::FG_MUTED))),
                    )
                    .child(div().flex_none().opacity(0.5).child(crate::pairing::dot(look, SharedString::from(format!("asleep-dot-{}", shelf.mac)), &chip)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .text_color(look.color(token::FG_MUTED))
                            .child(shelf.name.clone()),
                    )
                    .child(div().flex_none().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child(said)),
            );
        div()
            .flex_none()
            .flex()
            .mt(look.length(if after_shelf { token::SPACE_2 } else { token::SPACE_4 }))
            .child(labelled(strip, Role::Button, name))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jobs() -> Vec<SessionRecord> {
        crate::mock::jobs("jobs-many").expect("the fixture decodes")
    }

    fn air_asleep() -> Vec<crate::pairing::Unreachable> {
        air_why(crate::pairing::Why::Asleep { since: 1_700_000_000 })
    }

    fn air_why(why: crate::pairing::Why) -> Vec<crate::pairing::Unreachable> {
        vec![crate::pairing::Unreachable { id: "air".into(), name: "Air".into(), colour: ipc::MacColour::Orange, why, tip: None }]
    }

    /// The fixture with its first live Job that needs him and two quiet live ones on Air, and the id of the one that needs him.
    fn with_three_on_air() -> (Vec<SessionRecord>, String, Vec<String>) {
        let mut all = jobs();
        let needs = |job: &SessionRecord| job.mark.is_some_and(|mark| mark.lamp == JobLamp::Needs);
        let asking = all.iter().find(|job| job.state == SessionState::Live && needs(job) && job.snoozed_until.is_none()).expect("a Job that needs him").id.as_str().to_string();
        let others: Vec<String> = all.iter().filter(|job| job.state == SessionState::Live && job.id.as_str() != asking && !needs(job)).take(2).map(|job| job.id.as_str().to_string()).collect();
        let on_air: Vec<String> = std::iter::once(asking.clone()).chain(others).collect();
        for job in all.iter_mut().filter(|job| on_air.iter().any(|id| id == job.id.as_str())) {
            job.mac = Some("air".into());
        }
        (all, asking, on_air)
    }

    fn laid_out(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|line| match line {
                Line::Group(head) => format!("project {}", head.name),
                Line::Job(row) => format!("job {}", row.id),
                Line::Asleep(shelf) => format!("asleep {} {}", shelf.mac, if shelf.open { "open" } else { "shut" }),
            })
            .collect()
    }

    #[test]
    fn the_jobs_of_a_mac_asleep_leave_their_groups_for_one_fold_at_the_foot() {
        let (all, _, on_air) = with_three_on_air();
        let lines = lines_shelving(&all, &[], Scope::Active, "", &[], &Names::default(), None, &air_asleep());
        let drawn = laid_out(&lines);
        let fold = drawn.iter().position(|line| line.starts_with("asleep ")).expect("a fold for Air");
        assert_eq!(drawn.iter().filter(|line| line.starts_with("asleep ")).count(), 1, "one fold for the Mac");
        assert!(drawn[..fold].iter().all(|line| !on_air.iter().any(|id| *line == format!("job {id}"))), "none of them stays in a group: {drawn:?}");
        let under: Vec<&str> = drawn[fold + 1..].iter().map(String::as_str).collect();
        assert_eq!(under.len(), on_air.len(), "{drawn:?}");
        assert!(on_air.iter().all(|id| under.contains(&format!("job {id}").as_str())));
        assert!(lines[fold + 1..].iter().all(|line| matches!(line, Line::Job(row) if row.asleep)), "marked asleep, with no heading between");
        assert!(lines[..fold].iter().all(|line| !matches!(line, Line::Job(row) if row.asleep)));
    }

    #[test]
    fn the_jobs_of_a_mac_on_another_build_fold_the_same_way_and_say_so() {
        let (all, _, on_air) = with_three_on_air();
        for (why, said) in [(crate::pairing::Why::OlderBuild, "On an older build"), (crate::pairing::Why::NewerBuild, "On a newer build")] {
            let lines = lines_shelving(&all, &[], Scope::Active, "", &[], &Names::default(), None, &air_why(why));
            let drawn = laid_out(&lines);
            let fold = drawn.iter().position(|line| line.starts_with("asleep ")).expect("a fold for Air");
            assert_eq!(drawn.len() - fold - 1, on_air.len(), "{drawn:?}");
            assert!(lines[fold + 1..].iter().all(|line| matches!(line, Line::Job(row) if row.asleep)));
            assert_eq!(why.said(), said);
        }
        let older = ipc::Mac { id: "air".into(), name: "Air".into(), link: Some(ipc::MacLink::OlderBuild), why: Some("Air runs an older build.".into()), ..Default::default() };
        let found = crate::pairing::unreachable_of(&[older]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].tip.as_deref(), Some("Air runs an older build."), "a frozen link's tooltip is the Mac's own words");
        let this = ipc::Mac { id: "studio".into(), this_mac: true, link: Some(ipc::MacLink::OlderBuild), ..Default::default() };
        assert!(crate::pairing::unreachable_of(&[this]).is_empty(), "this Mac is never one");
    }

    #[test]
    fn the_fold_is_shut_by_default_and_hides_its_jobs_until_the_mac_is_opened() {
        let (all, _, on_air) = with_three_on_air();
        let lines = lines_shelving(&all, &[], Scope::Active, "", &[], &Names::default(), None, &air_asleep());
        let shut = laid_out(&shelve_open(lines.clone(), &BTreeSet::new()));
        assert_eq!(shut.last().map(String::as_str), Some("asleep air shut"));
        assert!(!shut.iter().any(|line| on_air.iter().any(|id| *line == format!("job {id}"))), "folded away: {shut:?}");
        let open: BTreeSet<String> = ["air".to_string()].into();
        let opened = laid_out(&shelve_open(lines, &open));
        let fold = opened.iter().position(|line| line == "asleep air open").expect("open");
        assert_eq!(opened.len() - fold - 1, on_air.len());
    }

    #[test]
    fn a_linked_macs_jobs_stand_in_their_groups_as_before() {
        let (all, _, _) = with_three_on_air();
        let before = lines_within(&all, &[], Scope::Active, "", &[], &Names::default(), None);
        let linked = lines_shelving(&all, &[], Scope::Active, "", &[], &Names::default(), None, &[]);
        assert_eq!(before, linked);
        assert!(!laid_out(&linked).iter().any(|line| line.starts_with("asleep ")));
        assert!(linked.iter().all(|line| !matches!(line, Line::Job(row) if row.asleep)));
    }

    #[test]
    fn a_job_on_a_mac_asleep_is_not_in_needs_you_and_not_in_a_fold_there() {
        let (all, asking, _) = with_three_on_air();
        let lines = lines_shelving(&all, &[], Scope::Needs, "", &[], &Names::default(), None, &air_asleep());
        assert!(!laid_out(&lines).iter().any(|line| *line == format!("job {asking}")), "{:?}", laid_out(&lines));
        let awake = lines_shelving(&all, &[], Scope::Needs, "", &[], &Names::default(), None, &[]);
        assert!(laid_out(&awake).iter().any(|line| *line == format!("job {asking}")), "it asks him once its Mac is awake");
    }

    #[test]
    fn a_job_on_a_mac_asleep_is_found_by_search_under_its_fold() {
        let (all, _, on_air) = with_three_on_air();
        let title = all.iter().find(|job| job.id.as_str() == on_air[1]).map(|job| title_of(job).to_string()).expect("a title");
        let lines = lines_shelving(&all, &[], Scope::Active, &title, &[], &Names::default(), None, &air_asleep());
        assert!(laid_out(&lines).iter().any(|line| line.starts_with("asleep ")));
        assert!(laid_out(&lines).iter().any(|line| *line == format!("job {}", on_air[1])));
    }

    #[gpui::test]
    fn the_keys_reach_the_fold_and_open_it_and_a_mac_that_wakes_gives_its_jobs_back(cx: &mut gpui::TestAppContext) {
        let (all, _, on_air) = with_three_on_air();
        cx.update(crate::init_app);
        cx.update(|cx| crate::pairing::hold(crate::pairing::mock_macs(true), cx));
        let list = cx.update(|cx| cx.new(|cx| JobsList::mock(&all, cx)));
        let shown = |list: &Entity<JobsList>, cx: &mut gpui::TestAppContext| list.read_with(cx, |list, _| laid_out(&list.lines));
        assert_eq!(shown(&list, cx).last().map(String::as_str), Some("asleep air shut"), "folded by default, the Jobs out of sight");
        // ↓ to the foot lands on the fold, which the keys carry by its own id.
        list.update(cx, |list, cx| {
            while let Some((_, id)) = list.next_row(1) {
                list.focused = Some(id);
            }
            assert_eq!(list.focused_shelf().map(|(mac, open)| (mac.to_string(), open)), Some(("air".to_string(), false)));
            // → unfolds it.
            list.fold_focused(false, cx);
        });
        let opened = shown(&list, cx);
        assert_eq!(opened.iter().filter(|line| on_air.iter().any(|id| **line == format!("job {id}"))).count(), on_air.len());
        // ← folds it again, and the keys stay on the fold.
        list.update(cx, |list, cx| list.fold_focused(true, cx));
        assert_eq!(shown(&list, cx).last().map(String::as_str), Some("asleep air shut"));
        list.update(cx, |list, cx| list.fold_focused(false, cx));
        // The Mac wakes: its Jobs are back in their groups and the fold is gone.
        cx.update(|cx| crate::pairing::scene("air-awake", cx).expect("a scene"));
        let woke = shown(&list, cx);
        assert!(!woke.iter().any(|line| line.starts_with("asleep ")), "{woke:?}");
        assert!(on_air.iter().all(|id| woke.contains(&format!("job {id}"))));
        list.read_with(cx, |list, _| assert!(list.asleep_open.is_empty()));
    }
}
