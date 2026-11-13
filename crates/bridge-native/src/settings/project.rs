//! The project's own values, drawn when Settings opens from the project switcher (his ruling, 10 Nov 2026).
//! Classic shows none, so these labels are new: each is the registry's word where it has one
//! (`packages/components/src/generated/vocabulary.ts`, `auto_merge`).
//!
//! **A save writes the whole block.** `POST /projects/project/:id/edit` replaces the project's values, so each
//! write reads the project fresh, changes one key, and sends the rest as it found them.

use gpui::{div, prelude::*, px, svg, AnyElement, App, Context, FocusHandle, Focusable, FontWeight, KeyDownEvent, Role, SharedString, Window};
use gpui_component::input::Input;
use ipc::{LookSet, ResolvedValue, ResolvedValues, ReviewsSet, ValueLayer, ValuesSet};

use super::overlay::{Picking, Scope, SettingsOverlay};
use super::view::{bone, bounds_of, card, ghost, mono, ringed, small, tip};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The values picked from a list, in the order the page draws them.
pub const CHOICES: [&str; 6] = ["pr_mode", "auto_merge", "plans", "pull_requests", "walks", "density"];
/// The values typed: the words go to Fleet as written.
pub const TYPED: [&str; 3] = ["review_model", "theme", "layout"];

/// A value's words, and how a person reads each.
pub fn options(name: &str) -> &'static [(&'static str, &'static str)] {
    match name {
        "pr_mode" => &[("ready", "Ready for review"), ("draft", "Draft")],
        "auto_merge" => &[("never", "A person merges"), ("checks-pass", "Fleet merges once the forge's checks pass"), ("always", "Fleet merges whatever ran")],
        "plans" | "pull_requests" => &[("off", "Off"), ("review", "Review"), ("review_and_approve", "Review and approve")],
        "walks" => &[("off", "Off"), ("review", "Review")],
        "density" => &[("compact", "Compact"), ("default", "Default"), ("roomy", "Roomy")],
        _ => &[],
    }
}

fn title(name: &str) -> &'static str {
    match name {
        "pr_mode" => "Opens pull requests as",
        "auto_merge" => "Merging",
        "review_model" => "Review model",
        "plans" => "Plans",
        "pull_requests" => "Pull requests",
        "walks" => "Walks",
        "theme" => "Theme",
        "layout" => "Layout",
        "density" => "Spacing",
        _ => "",
    }
}

/// Where a value the project does not set comes from. `None` for its own.
fn from_note(from: ValueLayer) -> Option<&'static str> {
    match from {
        ValueLayer::Project => None,
        ValueLayer::Default => Some("Armada's default"),
        ValueLayer::Machine => Some("From this machine"),
        ValueLayer::Kit => Some("From Kit"),
        ValueLayer::Repository => Some("From the repository"),
        ValueLayer::Job => Some("From the Job"),
    }
}

/// A value's word as it stands, and the layer it came from.
fn resolved(values: &ResolvedValues, name: &str) -> (Option<String>, ValueLayer) {
    let one = |value: &ResolvedValue| (value.value.clone(), value.from);
    let reviews = values.reviews.as_ref();
    match name {
        "pr_mode" => one(&values.pr_mode),
        "auto_merge" => one(&values.auto_merge),
        "review_model" => one(&values.review_model),
        "theme" => one(&values.theme),
        "layout" => one(&values.layout),
        "density" => one(&values.density),
        "plans" => reviews.map_or((Some("off".into()), ValueLayer::Default), |set| (Some(set.plans.clone()), set.from)),
        "pull_requests" => reviews.map_or((Some("off".into()), ValueLayer::Default), |set| (Some(set.pull_requests.clone()), set.from)),
        "walks" => reviews.map_or((Some("off".into()), ValueLayer::Default), |set| (Some(set.walks.clone()), set.from)),
        _ => (None, ValueLayer::Default),
    }
}

/// The project's own block with one value changed and every other key kept. An empty word unsets the value, so
/// it comes from the layer under the project again. A review kind sets all three, as a layer must: the two not
/// changed keep what they come to now.
pub fn with_value(own: Option<&ValuesSet>, now: Option<&ResolvedValues>, name: &str, word: &str) -> ValuesSet {
    let mut set = own.cloned().unwrap_or_default();
    let word = (!word.is_empty()).then(|| word.to_string());
    let mut look = set.look.clone().unwrap_or_default();
    match name {
        "pr_mode" => set.pr_mode = word.as_deref().and_then(ipc::PrMode::from_wire),
        "auto_merge" => set.auto_merge = word,
        "review_model" => set.review_model = word,
        "theme" => look.theme = word,
        "layout" => look.layout = word,
        "density" => look.density = word,
        "plans" | "pull_requests" | "walks" => {
            let held = |kind: &str| set.reviews.as_ref().and_then(|set| match kind {
                "plans" => set.plans.clone(),
                "pull_requests" => set.pull_requests.clone(),
                _ => set.walks.clone(),
            });
            let kind = |kind: &str| if kind == name { word.clone() } else { held(kind).or_else(|| now.and_then(|now| resolved(now, kind).0)) };
            return super::wire::with_reviews(Some(&set), ReviewsSet { plans: kind("plans"), pull_requests: kind("pull_requests"), walks: kind("walks") });
        }
        _ => {}
    }
    if matches!(name, "theme" | "layout" | "density") {
        set.look = (look != LookSet::default()).then_some(look);
    }
    set
}

fn typed_now(scope: &Scope, name: &str, cx: &App) -> Option<String> {
    scope.typed.get(name).map(|field| field.read(cx).value().trim().to_string())
}

/// Whether what is typed into a value differs from what it comes to.
fn dirty(scope: &Scope, name: &str, cx: &App) -> bool {
    let Some(values) = &scope.values else { return false };
    typed_now(scope, name, cx).is_some_and(|typed| typed != resolved(values, name).0.unwrap_or_default())
}

/// The keys the page's controls take, in the order Tab reaches them.
pub fn stops(scope: &Scope, cx: &App) -> Vec<FocusHandle> {
    if scope.values.is_none() {
        return Vec::new();
    }
    let mut stops = Vec::new();
    for name in ["pr_mode", "auto_merge", "review_model", "plans", "pull_requests", "walks", "theme", "layout", "density"] {
        if let Some(handle) = scope.stops.get(name) {
            stops.push(handle.clone());
        }
        if let Some(field) = scope.typed.get(name) {
            stops.push(field.read(cx).focus_handle(cx));
            if dirty(scope, name, cx) {
                stops.extend(scope.saves.get(name).cloned());
            }
        }
    }
    stops
}

fn from_line(look: &Look, from: ValueLayer) -> Option<AnyElement> {
    from_note(from).map(|said| small(look, said, look.color(token::FG_MUTED)).into_any_element())
}

fn heading(look: &Look, name: &str) -> AnyElement {
    div()
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(look.color(token::FG_DEFAULT))
        .child(title(name))
        .into_any_element()
}

fn choice(this: &SettingsOverlay, scope: &Scope, values: &ResolvedValues, name: &'static str, look: &Look, window: &Window, cx: &Context<SettingsOverlay>) -> AnyElement {
    let (word, from) = resolved(values, name);
    let word = word.unwrap_or_default();
    let shown = options(name).iter().find(|(one, _)| *one == word).map_or(word.clone(), |(_, said)| said.to_string());
    let off = !this.live || scope.saving.is_some();
    let Some(handle) = scope.stops.get(name) else { return div().into_any_element() };
    let trigger = ringed(
        look,
        div()
            .id(SharedString::from(format!("value-{name}")))
            .track_focus(handle)
            .flex()
            .items_center()
            .justify_between()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_SUNKEN))
            .text_size(look.length(token::TEXT_SM))
            .text_color(if off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_DEFAULT) })
            .when(!off, |trigger| trigger.cursor_pointer())
            .child(SharedString::from(shown))
            .child(svg().path(icon_path("chevron-down")).flex_none().size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED))),
        handle,
        window,
    )
    .when(!off, |trigger| {
        let open = move |this: &mut SettingsOverlay, from_keys: bool, window: &mut Window, cx: &mut Context<SettingsOverlay>| {
            let at = bounds_of(window, cx, Role::ComboBox, title(name)).map(|bounds| bounds.bottom_left()).unwrap_or_default();
            let now = this.scope.as_ref().and_then(|scope| scope.values.as_ref()).and_then(|values| resolved(values, name).0).unwrap_or_default();
            let options = options(name).iter().map(|(one, said)| (SharedString::from(*said), *one == now)).collect();
            this.open_choice(Picking::Value(name), title(name).into(), options, at, from_keys, window, cx);
        };
        trigger.on_click(cx.listener(move |this, _, window, cx| open(this, false, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space" | "down") {
                open(this, true, window, cx);
                cx.stop_propagation();
            }
        }))
    });
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_1))
        .max_w(look.length(token::W_PROPOSAL_SETTINGS))
        .child(heading(look, name))
        .child(labelled(trigger, Role::ComboBox, title(name)))
        .children(from_line(look, from))
        .into_any_element()
}

fn typed(this: &SettingsOverlay, scope: &Scope, values: &ResolvedValues, name: &'static str, look: &Look, window: &Window, cx: &Context<SettingsOverlay>) -> AnyElement {
    let (_, from) = resolved(values, name);
    let off = !this.live || scope.saving.is_some();
    let Some(field) = scope.typed.get(name) else { return div().into_any_element() };
    let save = (dirty(scope, name, cx) || scope.saving == Some(name)).then(|| scope.saves.get(name)).flatten().map(|handle| {
        let said = if scope.saving == Some(name) { "Saving…" } else { "Save" };
        let button = ringed(look, ghost(look, SharedString::from(format!("save-value-{name}")), said, off), handle, window).track_focus(handle).when(!off, |button| {
            button.on_click(cx.listener(move |this, _, window, cx| this.save_value(name, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.save_value(name, window, cx);
                }
            }))
        });
        labelled(button, Role::Button, format!("Save {}", title(name)))
    });
    let input = labelled(
        div()
            .id(SharedString::from(format!("value-line-{name}")))
            .key_context(crate::keymap::TEXT_BOX)
            .flex_1()
            .min_w_0()
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_SM))
            .child(Input::new(field).disabled(off)),
        Role::TextInput,
        title(name),
    );
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_1))
        .max_w(look.length(token::W_PROPOSAL_SETTINGS))
        .child(heading(look, name))
        .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(input).children(save))
        .children(from_line(look, from))
        .into_any_element()
}

fn tracker(values: &ResolvedValues, look: &Look) -> AnyElement {
    let (block, from) = values.tracker.as_ref().map_or((None, ValueLayer::Default), |one| (one.value.clone(), one.from));
    let said = match block.as_ref().and_then(|block| block.get("provider")) {
        Some(provider) => provider.clone(),
        None => "The forge's own".to_string(),
    };
    let rows = block.into_iter().flatten().filter(|(key, _)| key != "provider").map(|(key, value)| {
        div().flex().gap(look.length(token::SPACE_2)).font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(key)).child(SharedString::from(value))
    });
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_1))
        .child(labelled(div().id("value-tracker").text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).tooltip(tip("Set in the project's armada.yml")).child(SharedString::from(said.clone())), Role::Status, format!("Tracker: {said}")))
        .children(rows)
        .children(from_line(look, from))
        .into_any_element()
}

/// The page: its cards, or bones in their place while Fleet has not answered.
pub fn render(this: &SettingsOverlay, look: &Look, window: &Window, cx: &Context<SettingsOverlay>) -> Vec<AnyElement> {
    let Some(scope) = &this.scope else { return Vec::new() };
    let refused = scope.refused.clone().map(|said| labelled(small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id("values-refused"), Role::Alert, said).into_any_element());
    let Some(values) = &scope.values else {
        let bones = (0..3)
            .map(|n| {
                labelled(
                    div().id(SharedString::from(format!("values-bone-{n}"))).flex().flex_col().gap(look.length(token::SPACE_2)).child(bone(look, px(160.), look.length(token::LEADING_SM))).child(bone(look, px(480.), look.length(token::H_CONTROL))),
                    Role::Image,
                    "Loading",
                )
                .into_any_element()
            })
            .chain(refused)
            .collect();
        return vec![card(look, "values-card-loading", "Values", None, bones).into_any_element()];
    };
    let section = |id: &'static str, label: &'static str, body: Vec<AnyElement>| labelled(card(look, SharedString::from(format!("values-card-{id}")), label, None, body), Role::Region, label).into_any_element();
    let mut cards = vec![
        section("pull-requests", "Pull requests", vec![choice(this, scope, values, "pr_mode", look, window, cx), choice(this, scope, values, "auto_merge", look, window, cx), typed(this, scope, values, "review_model", look, window, cx)]),
        section("reviews", "Reviews", vec![choice(this, scope, values, "plans", look, window, cx), choice(this, scope, values, "pull_requests", look, window, cx), choice(this, scope, values, "walks", look, window, cx)]),
        section("look", "Look", vec![typed(this, scope, values, "theme", look, window, cx), typed(this, scope, values, "layout", look, window, cx), choice(this, scope, values, "density", look, window, cx)]),
        section("tracker", "Tracker", vec![tracker(values, look)]),
    ];
    if let Some(refused) = refused {
        cards.insert(0, refused);
    }
    cards
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::{PrMode, ResolvedReviews};

    fn own() -> ValuesSet {
        ValuesSet {
            pr_mode: PrMode::from_wire("draft"),
            auto_merge: Some("checks-pass".into()),
            review_model: Some("opus".into()),
            look: Some(LookSet { layout: None, theme: Some("dusk".into()), density: Some("roomy".into()) }),
            tracker: Some([("provider".to_string(), "linear".to_string())].into_iter().collect()),
            reviews: Some(ReviewsSet { plans: Some("review".into()), pull_requests: Some("off".into()), walks: Some("off".into()) }),
            ..ValuesSet::default()
        }
    }

    #[test]
    fn a_reviews_change_keeps_the_other_five_values_and_the_other_two_kinds() {
        let sent = with_value(Some(&own()), None, "walks", "review");
        let wanted = ReviewsSet { plans: Some("review".into()), pull_requests: Some("off".into()), walks: Some("review".into()) };
        assert_eq!(sent, ValuesSet { reviews: Some(wanted), ..own() });
    }

    #[test]
    fn a_first_review_kind_sets_all_three_from_what_they_come_to() {
        let now = ResolvedValues {
            pr_mode: ResolvedValue { value: Some("ready".into()), from: ValueLayer::Default, learned: false, inherited: None, inherited_from: None },
            auto_merge: ResolvedValue { value: None, from: ValueLayer::Default, learned: false, inherited: None, inherited_from: None },
            review_model: ResolvedValue { value: None, from: ValueLayer::Default, learned: false, inherited: None, inherited_from: None },
            layout: ResolvedValue { value: None, from: ValueLayer::Default, learned: false, inherited: None, inherited_from: None },
            theme: ResolvedValue { value: None, from: ValueLayer::Default, learned: false, inherited: None, inherited_from: None },
            density: ResolvedValue { value: None, from: ValueLayer::Default, learned: false, inherited: None, inherited_from: None },
            tracker: None,
            reviews: Some(ResolvedReviews { plans: "review_and_approve".into(), pull_requests: "review".into(), walks: "off".into(), from: ValueLayer::Kit, plans_from: None, pull_requests_from: None, walks_from: None, plans_inherited: None, plans_inherited_from: None, pull_requests_inherited: None, pull_requests_inherited_from: None, walks_inherited: None, walks_inherited_from: None }),
            autonomy: Vec::new(),
            ..ResolvedValues::default()
        };
        let sent = with_value(None, Some(&now), "plans", "off");
        assert_eq!(sent.reviews, Some(ReviewsSet { plans: Some("off".into()), pull_requests: Some("review".into()), walks: Some("off".into()) }));
    }

    #[test]
    fn an_emptied_value_is_unset_and_the_rest_kept() {
        let sent = with_value(Some(&own()), None, "theme", "");
        assert_eq!(sent, ValuesSet { look: Some(LookSet { layout: None, theme: None, density: Some("roomy".into()) }), ..own() });
        let sent = with_value(Some(&own()), None, "pr_mode", "ready");
        assert_eq!(sent, ValuesSet { pr_mode: PrMode::from_wire("ready"), ..own() });
    }
}
