//! Settings › Layout (`LayoutSettings`): each Cockpit panel, Job panel and place a layout arranges, with a switch for
//! whether it is drawn, then the layout mods. **His own choice outranks every layout mod**, and Reset to defaults
//! takes it back and switches every layout mod off. No region is ordered, so Classic draws no moves, and nor does this.

use std::collections::BTreeMap;

use gpui::{div, prelude::*, svg, AnyElement, App, Context, KeyDownEvent, Role, SharedString, Window};
use ipc::layout::Choice;
use ipc::ModKind;

use super::overlay::SettingsOverlay;
use super::view::{bone, card, ghost, ringed, switch_track, tip};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

pub struct Row {
    pub id: &'static str,
    pub label: &'static str,
    /// The registry glyph Classic's row draws.
    pub icon: &'static str,
    /// False for anything he has to decide on, or has to reach to undo a layout.
    pub hideable: bool,
}

pub struct Region {
    pub id: &'static str,
    pub label: &'static str,
    pub rows: &'static [Row],
}

const fn row(id: &'static str, label: &'static str, icon: &'static str, hideable: bool) -> Row {
    Row { id, label, icon, hideable }
}

/// `LAYOUT` in `packages/shell/src/layout.tsx`, in its order; the places are `PLACES`.
pub const REGIONS: [Region; 3] = [
    Region {
        id: "dashboard.panels",
        label: "Cockpit panels",
        rows: &[
            row("list", "Jobs", "box", false),
            row("field", "Field", "radar", true),
            row("merge-line", "Merge queue", "git-merge", true),
            row("activity", "Activity", "activity", true),
            row("bays", "Bays", "rows-3", true),
            row("controls", "Flight control", "panel-bottom", true),
            row("dispatch", "Dispatch", "send", true),
            row("autopilot", "Autopilot", "plane", true),
        ],
    },
    Region { id: "job.panels", label: "Job panels", rows: &[row("done", "Done", "circle-check", true), row("now", "Now", "circle-dot", false), row("next", "Open", "circle-dashed", true)] },
    Region {
        id: "rail",
        label: "Places",
        rows: &[
            row("overview", "Cockpit", "layout-dashboard", false),
            row("sessions", "Jobs", "square-terminal", true),
            row("kit", "Kit", "briefcase", true),
            row("mods", "Mods", "puzzle", false),
            row("settings", "Settings", "settings", false),
            row("guides", "Guides", "book-open", true),
            row("workflows", "Workflows", "workflow", true),
            row("studios", "Studios", "presentation", true),
            row("logbook", "Logbook", "notebook-tabs", true),
            row("reviews", "Reviews", "eye", true),
            row("issues", "Issues", "ticket", true),
            row("projects", "Projects", "list-todo", true),
        ],
    },
];

thread_local! {
    /// His own choices, as Fleet last said them: read once a connection, and again after each save.
    static HELD: std::cell::RefCell<BTreeMap<String, Choice>> = std::cell::RefCell::new(BTreeMap::new());
}

/// Take Fleet's `layout_choices` text, on a read or after a save. Text with a problem chooses nothing.
pub fn hold(text: &str) {
    HELD.with(|held| *held.borrow_mut() = ipc::layout::choices(text));
}

/// Whether he has switched this panel or place off (`useLayout(region).shown`). One a region cannot hide is always
/// drawn, whatever the text says.
pub fn hidden(region: &str, id: &str) -> bool {
    let hideable = REGIONS.iter().find(|one| one.id == region).and_then(|one| one.rows.iter().find(|row| row.id == id)).is_some_and(|row| row.hideable);
    hideable && HELD.with(|held| held.borrow().get(region).and_then(|one| one.hidden.as_ref()).is_some_and(|hidden| hidden.iter().any(|one| one == id)))
}

/// One layer of choices: a layout mod by its name, or his own (`None`).
type Layer = (Option<String>, BTreeMap<String, Choice>);

/// What a region hides once the layers are laid over each other, lowest first, and the mod that said so, where the
/// last layer to name the region's hidden list is a mod. A layer that names it replaces what was under it.
pub fn hidden_in(region: &str, layers: &[Layer]) -> (Vec<String>, Option<String>) {
    let mut hidden = Vec::new();
    let mut by = None;
    for (name, choices) in layers {
        if let Some(said) = choices.get(region).and_then(|one| one.hidden.clone()) {
            hidden = said;
            by = name.clone();
        }
    }
    (hidden, by)
}

/// What a layout mod's row opens to (`layoutPart` over `layoutOf`): what the first region it hides anything in hides,
/// by name, and that region. A file that hides nothing says the mod's name, in Layout.
pub fn layout_part(name: &str, text: &str) -> super::mods::Part {
    let choices = ipc::layout::choices(text);
    let first = REGIONS.iter().find_map(|region| choices.get(region.id).and_then(|one| one.hidden.clone()).filter(|hidden| !hidden.is_empty()).map(|hidden| (region, hidden)));
    let (hides, within) = match first {
        Some((region, hidden)) => (hidden.iter().map(|id| region.rows.iter().find(|one| one.id == id).map_or(id.as_str(), |one| one.label)).collect::<Vec<_>>().join(", "), region.label),
        None => (name.to_string(), "Layout"),
    };
    let fact = |label: &str, value: String| super::mods::Fact { label: label.into(), value, code: false, from: None };
    super::mods::Part { part: "Layout", facts: vec![fact("Hides", hides), fact("In", within.to_string())] }
}

impl SettingsOverlay {
    fn own_layout(&self) -> String {
        self.prefs.held().map(|held| held.layout_choices.clone()).unwrap_or_default()
    }

    /// The layout mods that are on, whose file Fleet checked.
    fn layout_mods_on(&self, cx: &App) -> Vec<String> {
        self.wire.mods(cx).iter().filter(|row| row.kind == Some(ModKind::Layout) && row.valid && super::mods::switched_on(row, &self.mods)).map(|row| row.name.clone()).collect()
    }

    fn layers(&self, cx: &App) -> Vec<Layer> {
        let mut layers: Vec<Layer> = self
            .layout_mods_on(cx)
            .into_iter()
            .filter_map(|name| {
                let text = self.mods.checked.get(&name).and_then(|one| one.layout.clone())?;
                Some((Some(name), ipc::layout::choices(&text)))
            })
            .collect();
        layers.push((None, ipc::layout::choices(&self.own_layout())));
        layers
    }

    /// A switch moved: his own choice for the region becomes what is drawn now, with the one row changed.
    fn layout_show(&mut self, region: &'static str, id: &'static str, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        let (hidden, _) = hidden_in(region, &self.layers(cx));
        let next: Vec<String> = if on { hidden.into_iter().filter(|one| one != id).collect() } else { hidden.into_iter().chain([id.to_string()]).collect() };
        let mut own = ipc::layout::choices(&self.own_layout());
        own.entry(region.to_string()).or_default().hidden = Some(next);
        self.save_pref("layout_choices", ipc::layout::text_of(&own), window, cx);
    }

    /// Whether anything differs from what ships: a choice of his, or a layout mod that is on.
    fn layout_changed(&self, cx: &App) -> bool {
        !self.own_layout().is_empty() || !self.layout_mods_on(cx).is_empty()
    }

    /// `toDefaults`: his choice taken back, and every layout mod switched off. The mods stay.
    fn layout_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.own_layout().is_empty() {
            self.save_pref("layout_choices", String::new(), window, cx);
        }
        for name in self.layout_mods_on(cx) {
            self.switch_mod(name, ModKind::Layout, false, window, cx);
        }
    }

    /// The page's stops, in the order it draws them.
    pub(super) fn layout_stops(&self, cx: &App) -> Vec<gpui::FocusHandle> {
        let mut stops: Vec<gpui::FocusHandle> = REGIONS.iter().flat_map(|region| region.rows.iter().filter(|row| row.hideable).filter_map(move |row| self.prefs.keys.get(&format!("layout:{}:{}", region.id, row.id)).cloned())).collect();
        stops.extend(self.prefs.keys.get("layout-reset").cloned());
        stops.extend(self.mod_stops(super::mods::Page::Layout, cx));
        stops
    }

    pub(super) fn render_layout(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Vec<AnyElement> {
        let body = match &self.prefs.read {
            None => div().flex().flex_col().gap(look.length(token::SPACE_3)).children((0..6).map(|n| bone(look, look.length(token::SPACE_6) * (8 - n % 3) as f32, look.length(token::LEADING_SM)))).into_any_element(),
            Some(Err(said)) => labelled(super::view::refusal(look, said.clone()).id("layout-unread"), Role::Alert, said.clone()).into_any_element(),
            Some(Ok(_)) => self.layout_regions(look, window, cx),
        };
        let mut cards = vec![labelled(card(look, "settings-card-layout", "Layout", None, vec![body]), Role::Region, "Layout").into_any_element()];
        cards.extend(self.render_mods(super::mods::Page::Layout, look, window, cx));
        cards
    }

    fn layout_regions(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let layers = self.layers(cx);
        let off = !self.live;
        let regions = REGIONS.iter().map(|region| {
            let (hidden, by) = hidden_in(region.id, &layers);
            let rows = region.rows.iter().map(|one| {
                let visible = !hidden.iter().any(|id| id == one.id);
                let mark = svg().flex_none().path(icon_path(one.icon)).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED));
                let label = div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(one.label);
                let control = if one.hideable {
                    self.layout_switch(region.id, one, visible, off, label, look, window, cx)
                } else {
                    labelled(div().id(SharedString::from(format!("layout-fixed-{}-{}", region.id, one.id))).flex_1().min_w_0().tooltip(tip("Always shown")).child(label), Role::Label, one.label).into_any_element()
                };
                // The puzzle names the mod that hid it; his own hiding says nothing.
                let from = by.as_ref().filter(|_| !visible).map(|name| {
                    let said = format!("Hidden by {name}");
                    labelled(div().id(SharedString::from(format!("layout-from-{}-{}", region.id, one.id))).flex_none().w(look.length(token::H_CONTROL)).flex().justify_center().tooltip(tip(name.clone())).child(svg().path(icon_path("puzzle")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED))), Role::Image, said).into_any_element()
                });
                // The puzzle's column is kept on every row, as wide as a layout mod's row holds after its switch, so
                // every switch on the page sits in one column.
                let trailing = div().flex_none().w(Self::trailing_wide(look)).flex().children(from);
                div().flex().items_center().gap(look.length(token::SPACE_3)).min_w_0().child(mark).child(control).child(trailing).into_any_element()
            });
            let caption = div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(region.label);
            labelled(div().id(SharedString::from(format!("layout-region-{}", region.id))).flex().flex_col().gap(look.length(token::SPACE_2)).child(caption).children(rows), Role::Group, region.label).into_any_element()
        });
        let changed = self.layout_changed(cx);
        let reset_off = off || !changed;
        let reset = self.prefs.keys.get("layout-reset").cloned().map(|focus| {
            let button = ringed(look, ghost(look, "layout-reset", "Reset to defaults", reset_off).border_color(look.color(token::BORDER_DEFAULT)), &focus, window).track_focus(&focus).when(!reset_off, |button| {
                button.on_click(cx.listener(|this, _, window, cx| this.layout_reset(window, cx))).on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.layout_reset(window, cx);
                    }
                }))
            });
            div().flex().child(labelled(button, Role::Button, "Reset to defaults")).into_any_element()
        });
        let refused = self.prefs.refused.get("layout_choices").map(|said| labelled(super::view::refusal(look, said.clone()).id("layout-refused"), Role::Alert, said.clone()).into_any_element());
        div().flex().flex_col().gap(look.length(token::SPACE_6)).children(regions).children(reset).children(refused).into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn layout_switch(&self, region: &'static str, one: &'static Row, visible: bool, off: bool, label: gpui::Div, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let id = one.id;
        let focus = self.prefs.keys.get(&format!("layout:{region}:{id}")).cloned();
        let switch = div()
            .id(SharedString::from(format!("layout-{region}-{id}")))
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .justify_between()
            .gap(look.length(token::SPACE_3))
            // Room inside the ring, taken back outside it so the label keeps its place.
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .mx(-look.length(token::SPACE_2))
            .my(-look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .child(label)
            .child(switch_track(look, visible, off))
            .when_some(focus, |row, focus| ringed(look, row.track_focus(&focus), &focus, window))
            .when(!off, |row| {
                row.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| this.layout_show(region, id, !visible, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.layout_show(region, id, !visible, window, cx);
                    }
                }))
            });
        labelled(switch, Role::Switch, one.label).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(name: Option<&str>, text: &str) -> Layer {
        (name.map(String::from), ipc::layout::choices(text))
    }

    #[test]
    fn a_held_choice_hides_only_what_its_region_may_hide() {
        hold(r#"{"version":1,"job.panels":{"hidden":["done","now"]},"dashboard.panels":{"hidden":["merge-line"]}}"#);
        assert!(hidden("job.panels", "done"));
        assert!(!hidden("job.panels", "now"), "Now cannot be hidden");
        assert!(!hidden("job.panels", "next"));
        assert!(hidden("dashboard.panels", "merge-line"));
        hold("not a layout");
        assert!(!hidden("job.panels", "done"), "text with a problem chooses nothing");
    }

    #[test]
    fn a_mod_hides_until_his_own_choice_names_the_region() {
        let hide_guides = layer(Some("hide-guides"), r#"{"version":1,"rail":{"hidden":["guides"]}}"#);
        assert_eq!(hidden_in("rail", &[hide_guides.clone(), layer(None, "")]), (vec!["guides".to_string()], Some("hide-guides".to_string())));
        let shown_again = layer(None, r#"{"version":1,"rail":{"hidden":[]}}"#);
        assert_eq!(hidden_in("rail", &[hide_guides, shown_again]), (Vec::new(), None), "his empty list shows what the mod hid");
    }

    #[test]
    fn a_layout_mod_opens_to_what_it_hides_and_where() {
        let facts = |part: super::super::mods::Part| part.facts.into_iter().map(|fact| (fact.label, fact.value)).collect::<Vec<_>>();
        let said = |label: &str, value: &str| (label.to_string(), value.to_string());
        assert_eq!(facts(layout_part("hide-guides", r#"{"version":1,"rail":{"hidden":["guides","kit"]}}"#)), [said("Hides", "Guides, Kit"), said("In", "Places")]);
        assert_eq!(facts(layout_part("bare", r#"{"version":1}"#)), [said("Hides", "bare"), said("In", "Layout")]);
    }

    #[test]
    fn every_row_is_one_the_shipped_registry_names() {
        for region in &REGIONS {
            let said = format!(r#"{{"version":1,"{}":{{"hidden":[{}]}}}}"#, region.id, region.rows.iter().map(|one| format!("\"{}\"", one.id)).collect::<Vec<_>>().join(","));
            assert_eq!(ipc::layout::problems(&said), Vec::<String>::new(), "{}", region.id);
            assert!(ipc::layout::regions().contains(&region.id.to_string()), "{}", region.id);
        }
    }
}
