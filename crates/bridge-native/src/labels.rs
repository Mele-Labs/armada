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

    /// A new entry on the window's page, drawn in whatever labelled element is open now.
    fn push(&mut self, id: WindowId, role: Role, name: SharedString, bounds: Bounds<Pixels>, shown: Bounds<Pixels>, radius: Option<Pixels>, painted: usize) -> usize {
        let parent = self.open.get(&id).and_then(|open| open.last().copied());
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
            self.child.prepaint(window, cx);
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
            self.child.paint(window, cx);
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
