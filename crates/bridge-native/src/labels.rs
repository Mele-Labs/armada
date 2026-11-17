//! Every element a person can press, read or focus carries a role and a name, and this is where
//! they are put on.
//!
//! [`labelled`] does two things with one pair. It sets the AccessKit role and label VoiceOver
//! reads (`.role`, `.aria_label`). It also writes down where the element was drawn this frame,
//! in the window's [`LabelBook`]. A walk aims each step at a `{role, name}`, and the check mode
//! plays it with no window on screen. gpui builds its AccessKit tree only while a screen reader is
//! connected, and its test platform has none, so walks read the book instead of that tree. Both
//! are written from the one pair, so they cannot disagree.

use std::collections::HashMap;
use std::panic;

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Role, SharedString, Stateful, StatefulInteractiveElement, Styled, Window, WindowId,
};

/// A role as a walk names it: `button`, `row`, `heading`. The walk's own words, which are
/// Electron's walk words too, map onto AccessKit's roles here and nowhere else.
pub fn role_named(word: &str) -> Option<Role> {
    Some(match word {
        "button" => Role::Button,
        "dialog" => Role::Dialog,
        "row" => Role::Row,
        "list" => Role::List,
        "listitem" => Role::ListItem,
        "heading" => Role::Heading,
        "textbox" => Role::TextInput,
        "region" => Role::Region,
        "article" => Role::Article,
        "form" => Role::Form,
        "radio" => Role::RadioButton,
        "checkbox" => Role::CheckBox,
        "group" => Role::Group,
        "radiogroup" => Role::RadioGroup,
        "switch" => Role::Switch,
        "slider" => Role::Slider,
        "toolbar" => Role::Toolbar,
        "status" => Role::Status,
        "timer" => Role::Timer,
        "label" => Role::Label,
        "link" => Role::Link,
        "menu" => Role::Menu,
        "menuitem" => Role::MenuItem,
        "menuitemcheckbox" => Role::MenuItemCheckBox,
        "searchbox" => Role::SearchInput,
        "img" => Role::Image,
        "combobox" => Role::ComboBox,
        "listbox" => Role::ListBox,
        "option" => Role::ListBoxOption,
        "progressbar" => Role::ProgressIndicator,
        "separator" => Role::Splitter,
        "alert" => Role::Alert,
        "tab" => Role::Tab,
        "tablist" => Role::TabList,
        "tooltip" => Role::Tooltip,
        "tree" => Role::Tree,
        "treeitem" => Role::TreeItem,
        "table" => Role::Table,
        _ => return None,
    })
}

/// One element drawn this frame, as a walk finds it.
#[derive(Clone, Debug)]
pub struct Drawn {
    pub role: Role,
    pub name: SharedString,
    pub bounds: Bounds<Pixels>,
    /// The part of it a scrolled or clipped parent lets show: what a walk rings.
    pub shown: Bounds<Pixels>,
    /// Its own corner radius, the largest of its four, so a walk's ring follows its shape. None where it sets none.
    pub radius: Option<Pixels>,
    /// The labelled element it is drawn in, by its place in the window's page. None for an element a
    /// deferred surface (an overlay, a menu) draws, which stands on its own over the screen.
    pub parent: Option<usize>,
    /// The names of the labelled elements it is drawn in, outermost first: where a note says it is.
    pub path: Vec<SharedString>,
    /// When it was painted this frame: one painted later and not inside it covers it.
    pub painted: usize,
}

/// What each window drew with a label, as of its last frame. Cleared by the window's
/// [`label_root`] before its children write, so a row that scrolled away is gone.
#[derive(Default)]
pub struct LabelBook {
    by_window: HashMap<WindowId, Vec<Drawn>>,
    /// The labelled elements being prepainted or painted now, innermost last.
    open: HashMap<WindowId, Vec<usize>>,
    /// How many labelled elements the window has painted this frame.
    painted: HashMap<WindowId, usize>,
    /// Windows whose root is prepainting its tree now: a label written outside it is on an overlay.
    in_tree: HashMap<WindowId, bool>,
    /// Whether an overlay wrote a label this frame, and as of the last frame drawn (`overlaid`).
    overlaid_now: HashMap<WindowId, bool>,
    overlaid_last: HashMap<WindowId, bool>,
}

impl gpui::Global for LabelBook {}

impl LabelBook {
    /// The first element in `window` with this role and name, in the order they were drawn.
    pub fn find(&self, window: WindowId, role: Role, name: &str) -> Option<&Drawn> {
        self.by_window.get(&window)?.iter().find(|drawn| drawn.role == role && drawn.name == name)
    }

    pub fn drawn(&self, window: WindowId) -> &[Drawn] {
        self.by_window.get(&window).map_or(&[], Vec::as_slice)
    }

    /// Whether an overlay (a menu, a popover) drew a label in `window`'s last frame. A view drawn from its
    /// cache cannot put an overlay's labels back (`kept`), so it is drawn whole while one stands.
    pub fn overlaid(&self, window: WindowId) -> bool {
        self.overlaid_last.get(&window).copied().unwrap_or(false)
    }

    /// A new entry on the window's page, drawn in whatever labelled element is open now.
    fn push(&mut self, id: WindowId, role: Role, name: SharedString, bounds: Bounds<Pixels>, shown: Bounds<Pixels>, radius: Option<Pixels>, painted: usize) -> usize {
        let parent = self.open.get(&id).and_then(|open| open.last().copied());
        if !self.in_tree.get(&id).copied().unwrap_or(false) {
            self.overlaid_now.insert(id, true);
        }
        self.push_in(id, parent, Drawn { role, name, bounds, shown, radius, parent, path: Vec::new(), painted })
    }

    /// `drawn` on the window's page inside `parent`, its path read off that parent.
    fn push_in(&mut self, id: WindowId, parent: Option<usize>, drawn: Drawn) -> usize {
        let Drawn { role, name, bounds, shown, radius, painted, .. } = drawn;
        let page = self.by_window.entry(id).or_default();
        let path = parent.map_or_else(Vec::new, |at| {
            let outer = &page[at];
            let mut path = outer.path.clone();
            if !outer.name.is_empty() {
                path.push(outer.name.clone());
            }
            path
        });
        page.push(Drawn { role, name, bounds, shown, radius, parent, path, painted });
        page.len() - 1
    }

    fn next_painted(&mut self, id: WindowId) -> usize {
        let count = self.painted.entry(id).or_default();
        *count += 1;
        *count
    }
}

/// Whether `outer` is `inner` or one of the labelled elements it is drawn in.
pub fn holds(drawn: &[Drawn], outer: usize, inner: usize) -> bool {
    let mut at = Some(inner);
    while let Some(one) = at {
        if one == outer {
            return true;
        }
        at = drawn.get(one).and_then(|one| one.parent);
    }
    false
}

/// Give `element` its role and name: what VoiceOver reads, and what a walk and a test aim at.
pub fn labelled<E>(mut element: Stateful<E>, role: Role, name: impl Into<SharedString>) -> Labelled
where
    Stateful<E>: StatefulInteractiveElement + IntoElement + Styled,
{
    let name = name.into();
    let corners = element.style().corner_radii.clone();
    let radius = [corners.top_left, corners.top_right, corners.bottom_right, corners.bottom_left];
    Labelled { role, name: name.clone(), child: element.role(role).aria_label(name).into_any_element(), root: false, radius, at: None }
}

/// Name a part a canvas paints, such as a node of a sketch, as `labelled` names an element: what a
/// walk aims at. Called while painting, so the part stands in the book until the next frame's.
pub fn note(role: Role, name: impl Into<SharedString>, bounds: Bounds<Pixels>, radius: Option<Pixels>, window: &Window, cx: &mut App) {
    let shown = bounds.intersect(&window.content_mask().bounds);
    let id = window.window_handle().window_id();
    let book = cx.default_global::<LabelBook>();
    let painted = book.next_painted(id);
    book.push(id, role, name.into(), bounds, shown, radius, painted);
}

/// The window's outermost element: it clears the window's page of the book, then draws `child`.
pub fn label_root(child: impl IntoElement) -> Labelled {
    Labelled { role: Role::Window, name: SharedString::default(), child: child.into_any_element(), root: true, radius: [None; 4], at: None }
}

/// A surface with no name of its own that hides what is under it, such as an overlay's scrim: no
/// note is written on it, and none on what it covers.
pub fn cover(child: impl IntoElement) -> Labelled {
    Labelled { role: Role::Unknown, name: SharedString::default(), child: child.into_any_element(), root: false, radius: [None; 4], at: None }
}

/// What a cached view wrote when it was last drawn whole: its labels, the notes its canvases painted,
/// and the web pages it painted. A frame drawn from gpui's cache runs none of that, so [`kept`] puts it
/// back, or the view's labels leave the book and its web pages are parked while a sibling pulses.
#[derive(Default)]
pub struct KeptView {
    labels: Vec<Took>,
    notes: Vec<Took>,
    pages: Vec<crate::web::Painted>,
    /// This frame: where its labels start on the page, and whether the view was drawn whole.
    at: usize,
    whole: bool,
}

#[derive(Clone)]
struct Took {
    drawn: Drawn,
    /// Its parent among the view's own labels, by place; None for the element the view is drawn in.
    inner: Option<usize>,
    /// When it was painted, counted from the view's first paint this frame; 0 where it was not.
    painted: usize,
}

/// `view`, an `Entity::cached` view, with what it wrote kept across the frames drawn from its cache.
/// `drawn` is a count the view's `render` adds one to, which is how a frame drawn whole is told apart.
pub fn kept(view: impl IntoElement, kept: std::rc::Rc<std::cell::RefCell<KeptView>>, drawn: std::rc::Rc<std::cell::Cell<u64>>) -> Kept {
    Kept { child: view.into_any_element(), kept, drawn }
}

pub struct Kept {
    child: AnyElement,
    kept: std::rc::Rc<std::cell::RefCell<KeptView>>,
    drawn: std::rc::Rc<std::cell::Cell<u64>>,
}

impl IntoElement for Kept {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Kept {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static panic::Location<'static>> {
        None
    }

    fn request_layout(&mut self, _id: Option<&GlobalElementId>, _inspector_id: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let id = window.window_handle().window_id();
        let book = cx.default_global::<LabelBook>();
        let at = book.by_window.entry(id).or_default().len();
        let outer = book.open.get(&id).and_then(|open| open.last().copied());
        let before = self.drawn.get();
        self.child.prepaint(window, cx);
        let whole = self.drawn.get() != before;
        let book = cx.default_global::<LabelBook>();
        let mut kept = self.kept.borrow_mut();
        kept.at = at;
        kept.whole = whole;
        if whole {
            let page = book.by_window.entry(id).or_default();
            kept.labels = page[at..].iter().map(|drawn| Took { drawn: drawn.clone(), inner: drawn.parent.filter(|parent| *parent >= at).map(|parent| parent - at), painted: 0 }).collect();
        } else {
            for took in &kept.labels {
                book.push_in(id, took.inner.map(|inner| at + inner).or(outer), took.drawn.clone());
            }
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let id = window.window_handle().window_id();
        let book = cx.default_global::<LabelBook>();
        let first = book.painted.get(&id).copied().unwrap_or(0);
        let from = book.by_window.entry(id).or_default().len();
        let outer = book.open.get(&id).and_then(|open| open.last().copied());
        let pages_from = crate::web::painted_mark(cx);
        self.child.paint(window, cx);
        let mut kept = self.kept.borrow_mut();
        let at = kept.at;
        let since = |painted: usize| if painted > first { painted - first } else { 0 };
        if kept.whole {
            let page = cx.default_global::<LabelBook>().by_window.entry(id).or_default();
            let count = kept.labels.len();
            for (place, took) in kept.labels.iter_mut().enumerate() {
                took.painted = page.get(at + place).map_or(0, |drawn| since(drawn.painted));
            }
            kept.notes = page[from..]
                .iter()
                .map(|drawn| Took { drawn: drawn.clone(), inner: drawn.parent.filter(|parent| (at..at + count).contains(parent)).map(|parent| parent - at), painted: since(drawn.painted) })
                .collect();
            kept.pages = crate::web::painted_since(pages_from, cx);
            return;
        }
        let book = cx.default_global::<LabelBook>();
        let mut last = first;
        let page = book.by_window.entry(id).or_default();
        for (place, took) in kept.labels.iter().enumerate() {
            if let Some(drawn) = page.get_mut(at + place).filter(|_| took.painted > 0) {
                drawn.painted = first + took.painted;
                last = last.max(drawn.painted);
            }
        }
        for note in &kept.notes {
            let mut drawn = note.drawn.clone();
            drawn.painted = first + note.painted;
            last = last.max(drawn.painted);
            book.push_in(id, note.inner.map(|inner| at + inner).or(outer), drawn);
        }
        book.painted.insert(id, last);
        let pages = kept.pages.clone();
        drop(kept);
        crate::web::paint_again(&pages, window, cx);
    }
}

pub struct Labelled {
    role: Role,
    name: SharedString,
    child: AnyElement,
    root: bool,
    radius: [Option<gpui::AbsoluteLength>; 4],
    /// Its place on the window's page this frame, from prepaint.
    at: Option<usize>,
}

impl IntoElement for Labelled {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Labelled {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let id = window.window_handle().window_id();
        let shown = bounds.intersect(&window.content_mask().bounds);
        let rem = window.rem_size();
        let radius = self.radius.iter().flatten().map(|corner| corner.to_pixels(rem)).reduce(Pixels::max);
        let book = cx.default_global::<LabelBook>();
        if self.root {
            book.by_window.entry(id).or_default().clear();
            book.open.entry(id).or_default().clear();
            book.painted.insert(id, 0);
            book.overlaid_now.insert(id, false);
            book.in_tree.insert(id, true);
            self.child.prepaint(window, cx);
            cx.default_global::<LabelBook>().in_tree.insert(id, false);
            return;
        }
        let at = book.push(id, self.role, self.name.clone(), bounds, shown, radius, 0);
        self.at = Some(at);
        book.open.entry(id).or_default().push(at);
        self.child.prepaint(window, cx);
        cx.default_global::<LabelBook>().open.entry(id).or_default().pop();
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(at) = self.at else {
            let id = window.window_handle().window_id();
            if self.root {
                cx.default_global::<LabelBook>().in_tree.insert(id, true);
            }
            self.child.paint(window, cx);
            // The window's root: the frame is drawn, so the web pages it did not paint are parked.
            if self.root {
                let book = cx.default_global::<LabelBook>();
                book.in_tree.insert(id, false);
                let overlaid = book.overlaid_now.get(&id).copied().unwrap_or(false);
                book.overlaid_last.insert(id, overlaid);
                crate::web::after_frame(window, cx);
            }
            return;
        };
        let id = window.window_handle().window_id();
        let book = cx.default_global::<LabelBook>();
        let painted = book.next_painted(id);
        if let Some(one) = book.by_window.get_mut(&id).and_then(|page| page.get_mut(at)) {
            one.painted = painted;
        }
        book.open.entry(id).or_default().push(at);
        self.child.paint(window, cx);
        cx.default_global::<LabelBook>().open.entry(id).or_default().pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{div, prelude::*, Context, Entity, TestAppContext, VisualTestContext};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// A view drawn from its cache: a labelled region with a labelled row in it.
    struct Held(Rc<Cell<u64>>);

    impl Render for Held {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.0.set(self.0.get() + 1);
            labelled(div().id("held").size_full().child(labelled(div().id("row").h(gpui::px(20.)), Role::Row, "Inner row")), Role::Region, "Held")
        }
    }

    /// Its sibling, drawn again each time it is told to, as a pulsing list row is.
    struct Pulse(Rc<Cell<u64>>);

    impl Render for Pulse {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.0.set(self.0.get() + 1);
            labelled(div().id("pulse").h(gpui::px(20.)), Role::Status, "Pulse")
        }
    }

    struct Root {
        held: Entity<Held>,
        pulse: Entity<Pulse>,
        kept: Rc<RefCell<KeptView>>,
        drawn: Rc<Cell<u64>>,
    }

    impl Render for Root {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let held = kept(self.held.clone().cached(gpui::StyleRefinement::default().size_full()), self.kept.clone(), self.drawn.clone());
            label_root(div().size_full().flex().flex_col().child(self.pulse.clone()).child(div().flex_1().child(held)))
        }
    }

    fn row(cx: &mut VisualTestContext) -> Option<(Vec<SharedString>, bool)> {
        cx.update(|window, cx| {
            let book = cx.try_global::<LabelBook>()?;
            let id = window.window_handle().window_id();
            let row = book.find(id, Role::Row, "Inner row")?;
            Some((row.path.clone(), row.painted > 0))
        })
    }

    #[gpui::test]
    fn a_view_drawn_from_its_cache_keeps_its_labels_in_the_book(cx: &mut TestAppContext) {
        let drawn = Rc::new(Cell::new(0));
        let pulsed = Rc::new(Cell::new(0));
        let (held_drawn, pulse_drawn) = (drawn.clone(), pulsed.clone());
        let (root, cx) = cx.add_window_view(move |_, cx| Root { held: cx.new(|_| Held(held_drawn.clone())), pulse: cx.new(|_| Pulse(pulse_drawn)), kept: Rc::default(), drawn: held_drawn });
        cx.run_until_parked();
        let (whole, first) = (drawn.get(), pulsed.get());
        assert_eq!(row(cx), Some((vec!["Held".into()], true)));
        for _ in 0..3 {
            root.update(cx, |root, cx| root.pulse.update(cx, |_, cx| cx.notify()));
            cx.run_until_parked();
        }
        assert!(pulsed.get() >= first + 3, "each pulse drew a frame");
        assert_eq!(drawn.get(), whole, "the held view is drawn from its cache while its sibling pulses");
        assert_eq!(row(cx), Some((vec!["Held".into()], true)), "its labels stand in the book, in the same place, painted");
    }
}
