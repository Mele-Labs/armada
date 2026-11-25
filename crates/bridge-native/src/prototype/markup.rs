//! A scene file, read as the browser reads markup and kept as a tree of the app's own parts.
//!
//! The markup is HTML-shaped, so an agent writes it the way it writes any page, and html5ever
//! (the parser `scene/mock_page.rs` uses) forgives what an agent gets wrong: an unclosed tag, an
//! attribute with no quotes. What it cannot forgive is a self-closing custom element, which HTML
//! reads as an open one: `<glyph name="x"/>` would swallow its next sibling. The two parts with no
//! content, `fixture` and `glyph`, are read as empty and what follows them is lifted out.

use std::collections::BTreeMap;

use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{parse_document, ParseOpts};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

/// Parts that hold no content.
const EMPTY: [&str; 2] = ["fixture", "glyph"];

/// One element of the scene: its part, its attributes and what it holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub tag: String,
    pub attrs: BTreeMap<String, String>,
    pub kids: Vec<Kid>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kid {
    Node(Node),
    Text(String),
}

impl Node {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(String::as_str)
    }

    /// An attribute that is written bare (`active`) or as `true`: present and not `false`.
    pub fn flag(&self, name: &str) -> bool {
        self.attrs.get(name).is_some_and(|value| value != "false")
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.kids.iter().filter_map(|kid| match kid {
            Kid::Node(node) => Some(node),
            Kid::Text(_) => None,
        })
    }

    /// The words directly in it, joined.
    pub fn words(&self) -> String {
        let mut said = String::new();
        for kid in &self.kids {
            if let Kid::Text(text) = kid {
                if !said.is_empty() {
                    said.push(' ');
                }
                said.push_str(text);
            }
        }
        said
    }
}

/// A `<fixture name src type>`: the data a scene binds, and the wire type its text must decode as.
#[derive(Clone, Debug, PartialEq)]
pub struct FixtureDecl {
    pub name: String,
    pub src: Option<String>,
    pub kind: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub title: String,
    /// The state drawn first, where the scene names one.
    pub first: Option<String>,
    /// The walk that shows the built screen, which `shot_check` plays to compare it with this scene.
    pub walk: Option<String>,
    pub fixtures: Vec<FixtureDecl>,
    /// The ids of the `<state>` parts, in the order they are written.
    pub states: Vec<String>,
    /// The same states as he reads them in the head: each one's name and what it shows.
    pub views: Vec<View>,
    pub root: Node,
}

/// A `<state id title says>` as the head's tile draws it: `title` names it, else its id does, and `says` is what it shows.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub id: String,
    pub title: String,
    pub says: Option<String>,
}

impl Scene {
    /// The state a scene opens on: the one it names, else the first it declares.
    pub fn opening(&self) -> Option<&str> {
        self.first.as_deref().filter(|first| self.states.iter().any(|one| one == first)).or_else(|| self.states.first().map(String::as_str))
    }
}

/// The scene in `markup`, or why there is none.
pub fn parse(markup: &str) -> Result<Scene, String> {
    let opts = ParseOpts { tree_builder: TreeBuilderOpts { scripting_enabled: false, ..Default::default() }, ..Default::default() };
    let dom = parse_document(RcDom::default(), opts).one(markup);
    let body = find(&dom.document, "body").ok_or("the markup has no body")?;
    let mut tops = body.children.borrow().iter().filter_map(node_of).collect::<Vec<_>>();
    let root = match tops.iter().position(|top| top.tag == "scene") {
        Some(at) => tops.swap_remove(at),
        None => return Err("the markup has no <scene>".to_string()),
    };
    let mut fixtures = Vec::new();
    let mut views = Vec::new();
    collect(&root, &mut fixtures, &mut views);
    let states = views.iter().map(|view: &View| view.id.clone()).collect();
    Ok(Scene {
        title: root.attr("title").unwrap_or_default().to_string(),
        first: root.attr("first").map(str::to_string),
        walk: root.attr("walk").map(str::to_string),
        fixtures,
        states,
        views,
        root,
    })
}

fn collect(at: &Node, fixtures: &mut Vec<FixtureDecl>, views: &mut Vec<View>) {
    for node in at.nodes() {
        match node.tag.as_str() {
            "fixture" => fixtures.push(FixtureDecl {
                name: node.attr("name").or_else(|| node.attr("src")).unwrap_or_default().to_string(),
                src: node.attr("src").map(str::to_string),
                kind: node.attr("type").map(str::to_string),
            }),
            "state" => {
                let id = node.attr("id").unwrap_or_default().to_string();
                let title = node.attr("title").map(str::trim).filter(|title| !title.is_empty()).map_or_else(|| id.clone(), str::to_string);
                let says = node.attr("says").map(str::trim).filter(|says| !says.is_empty()).map(str::to_string);
                views.push(View { id, title, says });
            }
            _ => {}
        }
        collect(node, fixtures, views);
    }
}

fn find(at: &Handle, local: &str) -> Option<Handle> {
    for child in at.children.borrow().iter() {
        if let NodeData::Element { name, .. } = &child.data {
            if &*name.local == local {
                return Some(child.clone());
            }
        }
        if let Some(found) = find(child, local) {
            return Some(found);
        }
    }
    None
}

/// `at` as a node, or nothing for a comment, a doctype or words that stand alone.
fn node_of(at: &Handle) -> Option<Node> {
    match kids_of(at).into_iter().next()? {
        Kid::Node(node) => Some(node),
        Kid::Text(_) => None,
    }
}

/// What `at` stands for in its parent: itself, then what an empty part held, lifted out beside it.
fn kids_of(at: &Handle) -> Vec<Kid> {
    match &at.data {
        NodeData::Text { contents } => {
            let words = contents.borrow().split_whitespace().collect::<Vec<_>>().join(" ");
            if words.is_empty() {
                Vec::new()
            } else {
                vec![Kid::Text(words)]
            }
        }
        NodeData::Element { name, attrs, .. } => {
            let tag = name.local.to_string();
            let attrs = attrs.borrow().iter().map(|attr| (attr.name.local.to_string(), attr.value.to_string())).collect();
            let held: Vec<Kid> = at.children.borrow().iter().flat_map(kids_of).collect();
            if EMPTY.contains(&tag.as_str()) {
                let mut kids = vec![Kid::Node(Node { tag, attrs, kids: Vec::new() })];
                kids.extend(held);
                kids
            } else {
                vec![Kid::Node(Node { tag, attrs, kids: held })]
            }
        }
        _ => Vec::new(),
    }
}
