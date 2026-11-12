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
}

/// What each window drew with a label, as of its last frame. Cleared by the window's
/// [`label_root`] before its children write, so a row that scrolled away is gone.
#[derive(Default)]
pub struct LabelBook {
    by_window: HashMap<WindowId, Vec<Drawn>>,
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
}

/// Give `element` its role and name: what VoiceOver reads, and what a walk and a test aim at.
pub fn labelled<E>(mut element: Stateful<E>, role: Role, name: impl Into<SharedString>) -> Labelled
where
    Stateful<E>: StatefulInteractiveElement + IntoElement + Styled,
{
    let name = name.into();
    let corners = element.style().corner_radii.clone();
    let radius = [corners.top_left, corners.top_right, corners.bottom_right, corners.bottom_left];
    Labelled { role, name: name.clone(), child: element.role(role).aria_label(name).into_any_element(), root: false, radius }
}

/// Name a part a canvas paints, such as a node of a sketch, as `labelled` names an element: what a
/// walk aims at. Called while painting, so the part stands in the book until the next frame's.
pub fn note(role: Role, name: impl Into<SharedString>, bounds: Bounds<Pixels>, radius: Option<Pixels>, window: &Window, cx: &mut App) {
    let shown = bounds.intersect(&window.content_mask().bounds);
    let page = cx.default_global::<LabelBook>().by_window.entry(window.window_handle().window_id()).or_default();
    page.push(Drawn { role, name: name.into(), bounds, shown, radius });
}

/// The window's outermost element: it clears the window's page of the book, then draws `child`.
pub fn label_root(child: impl IntoElement) -> Labelled {
    Labelled { role: Role::Window, name: SharedString::default(), child: child.into_any_element(), root: true, radius: [None; 4] }
}

pub struct Labelled {
    role: Role,
    name: SharedString,
    child: AnyElement,
    root: bool,
    radius: [Option<gpui::AbsoluteLength>; 4],
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
        let book = cx.default_global::<LabelBook>();
        let page = book.by_window.entry(id).or_default();
        if self.root {
            page.clear();
        } else {
            let shown = bounds.intersect(&window.content_mask().bounds);
            let rem = window.rem_size();
            let radius = self.radius.iter().flatten().map(|corner| corner.to_pixels(rem)).reduce(Pixels::max);
            page.push(Drawn { role: self.role, name: self.name.clone(), bounds, shown, radius });
        }
        self.child.prepaint(window, cx);
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
        self.child.paint(window, cx);
    }
}
