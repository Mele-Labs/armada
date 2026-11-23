//! What a scene file reads as, how it names tokens, which state it draws, what a fixture that
//! drifted from the wire says, and what each `data-part` is called.

use serde_json::Value;

use super::draw;
use super::fixtures::{Fixtures, Scope};
use super::markup::{self, FixtureDecl, Kid, Node};
use super::style::{self, Trouble};
use super::{Home, PrototypeView, Source};
use crate::labels::{label_root, LabelBook};
use crate::look::{token, Look};
use gpui::{div, prelude::*, Context, Entity, Role, TestAppContext, VisualTestContext, Window};

const JOBS: &str = r#"{"jobs":[{"title":"First","so_far":"Two files read"},{"title":"Second"},{"title":"Third","so_far":null}]}"#;

fn scene(body: &str) -> (markup::Scene, Fixtures, Vec<String>) {
    let scene = markup::parse(&format!("<scene title=\"t\"><fixture name=\"jobs\" type=\"json\"></fixture><state id=\"one\">{body}</state></scene>")).expect("the scene reads");
    let (fixtures, trouble) = Fixtures::read(&scene.fixtures, &[("jobs".to_string(), JOBS.to_string())]);
    (scene, fixtures, trouble)
}

fn bound(fixtures: &Fixtures, item: &Value, text: &str) -> String {
    let locals = [("job".to_string(), item)];
    let mut reads = Vec::new();
    Scope { fixtures, locals: &locals }.bind(text, &mut reads)
}

#[test]
fn a_field_missing_on_one_item_draws_empty_and_is_no_trouble() {
    let (scene, fixtures, _) = scene(r#"<list each="jobs.jobs" as="job"><row title="{job.title}" says="{job.so_far}"></row></list>"#);
    let without: Value = serde_json::json!({"title": "Second"});
    assert_eq!(bound(&fixtures, &without, "{job.so_far}"), "");
    assert_eq!(bound(&fixtures, &without, "Now {job.so_far}."), "Now .");
    let trouble = draw::check(&Look::dark(), &scene, &fixtures).into_lines();
    assert!(trouble.is_empty(), "{trouble:?}");
}

#[test]
fn a_path_matching_no_item_goes_to_trouble() {
    let (scene, fixtures, _) = scene(r#"<list each="jobs.jobs" as="job"><row title="{job.title}" says="{job.so_farr}"></row></list><text>{jobs.jobs.0.titel}</text>"#);
    let trouble = draw::check(&Look::dark(), &scene, &fixtures).into_lines();
    assert_eq!(trouble, ["`{job.so_farr}` matches no field in jobs", "`{jobs.jobs.0.titel}` matches no field in jobs"]);
}

/// Every part the reader names, with attributes and words, two states, and chrome outside them.
const WHOLE: &str = r#"<scene title="Jobs" first="two">
  <fixture name="jobs" src="jobs.json" type="jobs"></fixture>
  <tabs data-part="View tabs"><tab goes="one">One</tab><tab goes="two" active>Two</tab></tabs>
  <state id="one">
    <pane title="Jobs"><list each="jobs.sessions" as="job" limit="2"><row title="{job.title}" says="so far"><chip tone="running" glyph="circle-dot">live</chip></row></list></pane>
  </state>
  <state id="two">
    <card title="Detail" tone="accent"><box dir="row" gap="2"><glyph name="git-branch"></glyph><text style="muted">Two files read</text><button kind="primary" goes="one">Back</button></box></card>
  </state>
</scene>"#;

fn tags(node: &Node) -> Vec<&str> {
    node.nodes().map(|kid| kid.tag.as_str()).collect()
}

fn only<'a>(node: &'a Node, tag: &str) -> &'a Node {
    node.nodes().find(|kid| kid.tag == tag).unwrap_or_else(|| panic!("no <{tag}> in <{}>", node.tag))
}

#[test]
fn a_scene_reads_as_the_part_tree_its_markup_writes() {
    let scene = markup::parse(WHOLE).expect("the scene reads");
    assert_eq!((scene.title.as_str(), scene.first.as_deref()), ("Jobs", Some("two")));
    assert_eq!(scene.states, ["one", "two"]);
    assert_eq!(scene.fixtures, [FixtureDecl { name: "jobs".into(), src: Some("jobs.json".into()), kind: Some("jobs".into()) }]);
    // The empty fixture does not swallow what follows it, and what is outside the states is the scene's own chrome.
    assert_eq!(tags(&scene.root), ["fixture", "tabs", "state", "state"]);
    let tabs = only(&scene.root, "tabs");
    assert_eq!(tabs.attr("data-part"), Some("View tabs"));
    let tab: Vec<_> = tabs.nodes().map(|tab| (tab.attr("goes"), tab.flag("active"), tab.words())).collect();
    assert_eq!(tab, [(Some("one"), false, "One".to_string()), (Some("two"), true, "Two".to_string())]);

    let states: Vec<_> = scene.root.nodes().filter(|node| node.tag == "state").collect();
    let list = only(only(states[0], "pane"), "list");
    assert_eq!(only(states[0], "pane").attr("title"), Some("Jobs"));
    assert_eq!((list.attr("each"), list.attr("as"), list.attr("limit")), (Some("jobs.sessions"), Some("job"), Some("2")));
    let row = only(list, "row");
    assert_eq!((row.attr("title"), row.attr("says")), (Some("{job.title}"), Some("so far")));
    let chip = only(row, "chip");
    assert_eq!((chip.attr("tone"), chip.attr("glyph"), chip.words().as_str()), (Some("running"), Some("circle-dot"), "live"));

    let card = only(states[1], "card");
    assert_eq!((card.attr("title"), card.attr("tone")), (Some("Detail"), Some("accent")));
    let inner = only(card, "box");
    assert_eq!(tags(inner), ["glyph", "text", "button"], "the empty glyph keeps what follows it as its sibling, in order");
    assert_eq!(only(inner, "glyph").kids, Vec::<Kid>::new());
    assert_eq!(only(inner, "glyph").attr("name"), Some("git-branch"));
    assert_eq!((only(inner, "text").attr("style"), only(inner, "text").words().as_str()), (Some("muted"), "Two files read"));
    let button = only(inner, "button");
    assert_eq!((button.attr("kind"), button.attr("goes"), button.words().as_str()), (Some("primary"), Some("one"), "Back"));
}

/// The first child of the scene's root, read from `<scene><box {attrs}></box></scene>`.
fn boxed(attrs: &str) -> Node {
    let scene = markup::parse(&format!("<scene title=\"t\"><box {attrs}></box></scene>")).expect("the scene reads");
    only(&scene.root, "box").clone()
}

#[test]
fn a_space_a_radius_and_a_colour_named_in_a_scene_resolve_to_the_themes_own() {
    let look = Look::dark();
    let mut trouble = Trouble::default();
    let read = style::read(&look, &boxed(r#"pad="3 2" gap="2" radius="md" fill="bg-raised" edge="border-subtle""#), &mut trouble);
    assert_eq!(trouble.lines(), Vec::<String>::new());
    assert_eq!(read.pad, Some((look.length_named("space-3").unwrap(), look.length_named("space-2").unwrap())));
    assert_eq!(read.gap, look.length_named("space-2").ok());
    assert_eq!(read.radius, look.length_named("radius-md").ok());
    assert_eq!(read.fill, Some(look.color(token::BG_RAISED)));
    assert_eq!(read.edge, Some(look.color(token::BORDER_SUBTLE)));
    let tone = style::read(&look, &boxed(r#"fill="success""#), &mut trouble);
    assert_eq!(tone.fill, Some(crate::parts::Tone::Success.ink(&look)), "a tone word is a colour too");
    assert!(trouble.lines().is_empty());
}

#[test]
fn a_space_a_radius_or_a_colour_the_theme_has_no_name_for_goes_to_trouble_and_is_never_replaced_quietly() {
    let look = Look::dark();
    let mut trouble = Trouble::default();
    let read = style::read(&look, &boxed(r#"pad="99" gap="huge" radius="round" fill="bg-raisd" edge="nope""#), &mut trouble);
    assert_eq!(
        trouble.lines(),
        [
            "<box> pad=\"99\" is no space step",
            "<box> gap=\"huge\" is no space step",
            "<box> radius=\"round\" is not sm, md, lg, full or points",
            "<box> fill=\"bg-raisd\" is no colour token",
            "<box> edge=\"nope\" is no colour token",
        ]
    );
    assert_eq!(read.radius, None, "an unknown radius is left unset, not rounded to a guess");
    assert_eq!(read.fill, Some(look.unknown_marker()), "an unknown colour draws as the marker");
    assert_eq!(read.edge, Some(look.unknown_marker()));
    assert_ne!(look.unknown_marker(), look.color(token::BG_RAISED));
}

/// A scene with two states, the first named by `first` (or not named when empty), and a tab that goes to `goes`.
fn two_states(first: &str, goes: &str) -> String {
    let first = if first.is_empty() { String::new() } else { format!(" first=\"{first}\"") };
    format!("<scene title=\"t\"{first}><tabs><tab goes=\"{goes}\">Go</tab></tabs><state id=\"one\"><text>One</text></state><state id=\"two\"><text>Two</text></state></scene>")
}

fn source(markup: String, state: Option<&str>) -> Source {
    Source { scene: "s.html".into(), markup, fixtures: Vec::new(), state: state.map(str::to_string), trouble: None }
}

fn view(cx: &mut TestAppContext, source: Source) -> Entity<PrototypeView> {
    cx.update(|cx| cx.set_global(Look::dark()));
    cx.new(|cx| PrototypeView::new(source, Home::Tab, cx))
}

fn shown(view: &Entity<PrototypeView>, cx: &TestAppContext) -> Option<String> {
    view.read_with(cx, |view, _| view.state().map(str::to_string))
}

fn trouble_of(view: &Entity<PrototypeView>, cx: &TestAppContext) -> Vec<String> {
    view.read_with(cx, |view, _| view.trouble().to_vec())
}

#[gpui::test]
fn a_scene_opens_on_the_state_it_names_else_on_the_first_it_declares(cx: &mut TestAppContext) {
    let named = view(cx, source(two_states("two", "one"), None));
    assert_eq!(shown(&named, cx).as_deref(), Some("two"));
    let unnamed = view(cx, source(two_states("", "one"), None));
    assert_eq!(shown(&unnamed, cx).as_deref(), Some("one"));
    let undeclared = view(cx, source(two_states("three", "one"), None));
    assert_eq!(shown(&undeclared, cx).as_deref(), Some("one"), "a `first` no state answers to falls back to the first declared");
}

#[gpui::test]
fn the_state_the_wire_names_moves_the_drawing_and_a_save_without_one_keeps_what_he_pressed(cx: &mut TestAppContext) {
    let markup = two_states("one", "two");
    let at = view(cx, source(markup.clone(), None));
    assert_eq!(shown(&at, cx).as_deref(), Some("one"));
    at.update(cx, |view, cx| view.set_source(source(markup.clone(), Some("two")), cx));
    assert_eq!(shown(&at, cx).as_deref(), Some("two"), "the agent's scene_state moves the drawing");
    at.update(cx, |view, cx| view.go("one", cx));
    at.update(cx, |view, cx| view.set_source(source(format!("{markup}<!-- saved again -->"), Some("two")), cx));
    assert_eq!(shown(&at, cx).as_deref(), Some("one"), "the same scene_state saved again leaves the state he pressed");
    assert!(trouble_of(&at, cx).is_empty(), "{:?}", trouble_of(&at, cx));
}

#[gpui::test]
fn a_press_that_goes_to_a_state_the_scene_has_not_is_refused_and_one_it_has_moves_to_it(cx: &mut TestAppContext) {
    let at = view(cx, source(two_states("one", "two"), None));
    at.update(cx, |view, cx| view.go("nowhere", cx));
    assert_eq!(shown(&at, cx).as_deref(), Some("one"), "a state nothing declares is not drawn");
    at.update(cx, |view, cx| view.go("two", cx));
    assert_eq!(shown(&at, cx).as_deref(), Some("two"));
}

#[gpui::test]
fn a_scene_state_the_scene_does_not_declare_is_trouble_and_leaves_the_drawing_where_it_was(cx: &mut TestAppContext) {
    let at = view(cx, source(two_states("one", "two"), None));
    at.update(cx, |view, cx| view.set_source(source(two_states("one", "two"), Some("elsewhere")), cx));
    assert_eq!(shown(&at, cx).as_deref(), Some("one"));
    assert_eq!(trouble_of(&at, cx), ["The state elsewhere the agent set is not one the scene declares"]);
}

#[test]
fn a_goes_that_names_no_state_goes_to_trouble() {
    let scene = markup::parse(&two_states("one", "nowhere")).expect("the scene reads");
    let trouble = draw::check(&Look::dark(), &scene, &Fixtures::default()).into_lines();
    assert_eq!(trouble, ["<tab> goes=\"nowhere\" names no state"]);
    let fine = markup::parse(&two_states("one", "two")).expect("the scene reads");
    assert!(draw::check(&Look::dark(), &fine, &Fixtures::default()).into_lines().is_empty());
}

#[test]
fn a_fixture_that_does_not_decode_as_its_type_puts_the_wire_error_in_the_trouble_line() {
    let decls = [FixtureDecl { name: "jobs".into(), src: Some("jobs.json".into()), kind: Some("jobs".into()) }];
    let bad = r#"{"sessions": "not a list"}"#;
    let (fixtures, trouble) = Fixtures::read(&decls, &[("jobs.json".to_string(), bad.to_string())]);
    let wire = ipc::decode::<ipc::SessionList>("SessionList", bad.as_bytes()).expect_err("the wire refuses it").to_string();
    assert!(wire.starts_with("not a readable SessionList: "), "{wire}");
    assert_eq!(trouble, [format!("Fixture jobs: {wire}")]);
    assert!(fixtures.value("jobs").is_none(), "what did not decode binds to nothing");
    let (fixtures, trouble) = Fixtures::read(&decls, &[("jobs.json".to_string(), r#"{"sessions":[]}"#.to_string())]);
    assert!(trouble.is_empty(), "{trouble:?}");
    assert!(fixtures.value("jobs").is_some());
}

#[gpui::test]
fn a_fixture_that_drifted_from_the_wire_shows_in_the_views_trouble_line(cx: &mut TestAppContext) {
    let markup = r#"<scene title="t"><fixture name="jobs" src="jobs.json" type="jobs"></fixture><state id="one"><text>{jobs.sessions}</text></state></scene>"#;
    let bad = r#"{"sessions": "not a list"}"#;
    let wire = ipc::decode::<ipc::SessionList>("SessionList", bad.as_bytes()).expect_err("the wire refuses it").to_string();
    let at = view(cx, Source { fixtures: vec![("jobs.json".into(), bad.into())], ..source(markup.into(), None) });
    assert_eq!(trouble_of(&at, cx)[0], format!("Fixture jobs: {wire}"));
}

/// A scene whose parts each carry a `data-part`, one bound to its list item.
const NAMED: &str = r#"<scene title="t" first="one">
  <fixture name="jobs" type="json"></fixture>
  <tabs data-part="View tabs"><tab goes="one" data-part="One tab">One</tab><tab goes="two" data-part="Two tab">Two</tab></tabs>
  <state id="one">
    <pane title="Jobs" data-part="Jobs pane">
      <list each="jobs.items" as="job" data-part="Jobs list"><row title="{job.title}" data-part="Row {job.title}"><chip data-part="Mark {job.title}">live</chip></row></list>
    </pane>
  </state>
  <state id="two">
    <card title="Detail" data-part="Detail card"><text data-part="Summary">Words</text><button data-part="Back button" goes="one">Back</button><glyph name="git-branch" data-part="Branch mark"></glyph></card>
  </state>
</scene>"#;

struct Host(Entity<PrototypeView>);

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        label_root(div().size_full().child(self.0.clone()))
    }
}

fn names(cx: &mut VisualTestContext) -> Vec<(Role, String)> {
    cx.run_until_parked();
    cx.update(|window, cx| {
        let book = cx.global::<LabelBook>();
        book.drawn(window.window_handle().window_id()).iter().map(|drawn| (drawn.role, drawn.name.to_string())).collect()
    })
}

#[gpui::test]
fn each_data_part_is_a_labelled_element_named_as_the_scene_binds_it(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_global(Look::dark()));
    let source = Source { fixtures: vec![("jobs".into(), r#"{"items":[{"title":"Alpha"},{"title":"Beta"}]}"#.into())], ..source(NAMED.into(), None) };
    let (host, cx) = cx.add_window_view(move |_, cx| Host(cx.new(|cx| PrototypeView::new(source, Home::Shot, cx))));
    let found = names(cx);
    for want in [
        (Role::TabList, "View tabs"),
        (Role::Tab, "One tab"),
        (Role::Tab, "Two tab"),
        (Role::Region, "Jobs pane"),
        (Role::List, "Jobs list"),
        (Role::Row, "Row Alpha"),
        (Role::Row, "Row Beta"),
        (Role::Status, "Mark Alpha"),
    ] {
        assert!(found.contains(&(want.0, want.1.to_string())), "{want:?} is not among {found:?}");
    }
    assert!(!found.iter().any(|(_, name)| name == "Detail card"), "a part of a state not drawn is not in the book");
    host.update(cx, |host, cx| host.0.update(cx, |view, cx| view.go("two", cx)));
    let found = names(cx);
    for want in [(Role::Group, "Detail card"), (Role::Label, "Summary"), (Role::Button, "Back button"), (Role::Image, "Branch mark")] {
        assert!(found.contains(&(want.0, want.1.to_string())), "{want:?} is not among {found:?}");
    }
    assert!(!found.iter().any(|(_, name)| name == "Jobs pane"), "the state left is no longer drawn");
}

#[gpui::test]
fn a_press_on_a_part_that_goes_to_a_state_draws_that_state(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_global(Look::dark()));
    let source = Source { fixtures: vec![("jobs".into(), r#"{"items":[]}"#.into())], ..source(NAMED.into(), None) };
    let (host, cx) = cx.add_window_view(move |_, cx| Host(cx.new(|cx| PrototypeView::new(source, Home::Shot, cx))));
    let _ = names(cx);
    let at = |cx: &mut VisualTestContext, name: &str| {
        cx.update(|window, cx| cx.global::<LabelBook>().find(window.window_handle().window_id(), Role::Tab, name).map(|drawn| drawn.bounds.center()))
    };
    let opening = host.read_with(cx, |host, cx| host.0.read(cx).state().map(str::to_string));
    assert_eq!(opening.as_deref(), Some("one"));
    let point = at(cx, "Two tab").expect("the tab is drawn");
    cx.simulate_click(point, gpui::Modifiers::none());
    let moved = host.read_with(cx, |host, cx| host.0.read(cx).state().map(str::to_string));
    assert_eq!(moved.as_deref(), Some("two"), "a click on the tab that goes to two draws two");
}
