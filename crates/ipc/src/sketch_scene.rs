//! A sketch's scene as a client draws it: `scene.ts`'s `parseScene` in Rust, so the native Bridge
//! reads a [`RawScene`] without decoding JSON itself. **All of it or nothing**, as there: a scene that
//! does not read is `None` and draws nothing.

use std::collections::HashSet;

use serde::Deserialize;

use crate::RawScene;

/// The registry glyphs a node may carry (`SCENE_ICONS`). Anything else fails the scene.
pub const SCENE_ICONS: [&str; 25] = [
    "activity", "bot", "box", "clock", "cpu", "eye", "file", "git-branch", "git-merge", "globe", "hammer", "hard-drive", "layers", "lock",
    "package", "rocket", "scale", "scroll-text", "server", "settings", "shield-check", "terminal", "webhook", "wrench", "zap",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneKind {
    Box,
    Group,
    Lane,
    Label,
    Code,
    Wire,
    Screen,
    Mock,
}

/// What a wireframe node stands for, drawn as a block and never as a control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WirePart {
    Button,
    Input,
    List,
    Card,
    Nav,
    Text,
}

/// A `box`'s outline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneShape {
    Box,
    Pill,
    Circle,
    Diamond,
    Cylinder,
    Hexagon,
}

/// The colours a node or an arrow may be drawn in, by name: `--sketch-<name>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneColor {
    Blue,
    Teal,
    Green,
    Amber,
    Orange,
    Red,
    Grey,
}

/// How an arrow's line is drawn. Solid where unset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneLine {
    Solid,
    Dashed,
    Dotted,
}

/// Where an arrow carries its head. At the end where unset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneHead {
    End,
    Start,
    Both,
    /// Named `none` on the wire; spelled out so it never reads as `Option::None`.
    #[serde(rename = "none")]
    NoHead,
}

/// How an arrow runs between its nodes. An elbow where unset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenePath {
    Elbow,
    Straight,
    Curve,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SceneNode {
    pub id: String,
    pub kind: SceneKind,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub w: Option<f64>,
    #[serde(default)]
    pub h: Option<f64>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub wire: Option<WirePart>,
    #[serde(default)]
    pub step: Option<f64>,
    #[serde(default)]
    pub shape: Option<SceneShape>,
    #[serde(default)]
    pub color: Option<SceneColor>,
    /// A `mock`'s HTML, its own rules and the stylesheets it is drawn with. Drawn once pages are (#20).
    #[serde(default)]
    pub html: Option<String>,
    #[serde(default)]
    pub css: Option<String>,
    #[serde(default)]
    pub styles: Option<Vec<String>>,
    #[serde(default)]
    pub kept: Option<String>,
    /// A `screen`'s drawing, read whole when screens are drawn.
    #[serde(default)]
    pub screen: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SceneEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub flow: Option<bool>,
    #[serde(default)]
    pub step: Option<f64>,
    #[serde(default)]
    pub color: Option<SceneColor>,
    #[serde(default)]
    pub line: Option<SceneLine>,
    #[serde(default)]
    pub head: Option<SceneHead>,
    #[serde(default)]
    pub path: Option<ScenePath>,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct ScenePoint {
    pub x: f64,
    pub y: f64,
}

/// A hand-drawn line, as the owner's pad keeps it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SceneStroke {
    pub id: String,
    pub points: Vec<ScenePoint>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Scene {
    pub nodes: Vec<SceneNode>,
    pub edges: Vec<SceneEdge>,
    #[serde(default)]
    pub strokes: Vec<SceneStroke>,
}

impl RawScene {
    /// The scene, where every node, edge and stroke reads; ids are unique per list and every edge
    /// joins two nodes the scene holds. `None` otherwise.
    pub fn scene(&self) -> Option<Scene> {
        let scene: Scene = serde_json::from_value(self.0.clone()).ok()?;
        let ids: HashSet<&str> = scene.nodes.iter().map(|node| node.id.as_str()).collect();
        let edge_ids: HashSet<&str> = scene.edges.iter().map(|edge| edge.id.as_str()).collect();
        let nodes_read = scene.nodes.iter().all(|node| {
            !node.id.is_empty()
                && node.x.is_finite()
                && node.y.is_finite()
                && node.icon.as_deref().is_none_or(|icon| SCENE_ICONS.contains(&icon))
                && (!matches!(node.kind, SceneKind::Group | SceneKind::Lane) || (node.w.is_some() && node.h.is_some()))
                && (node.kind != SceneKind::Mock || (node.html.as_deref().is_some_and(|html| !html.trim().is_empty()) && node.w.is_some()))
                && (node.kind != SceneKind::Screen || node.screen.is_some())
        });
        let edges_read = scene.edges.iter().all(|edge| !edge.id.is_empty() && ids.contains(edge.from.as_str()) && ids.contains(edge.to.as_str()));
        (nodes_read && edges_read && ids.len() == scene.nodes.len() && edge_ids.len() == scene.edges.len()).then_some(scene)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(text: &str) -> RawScene {
        RawScene(serde_json::from_str(text).expect("json"))
    }

    #[test]
    fn a_scene_reads_whole_or_not_at_all() {
        let good = raw(r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"A","icon":"bot"},{"id":"b","kind":"group","x":0,"y":0,"w":10,"h":10}],"edges":[{"id":"e","from":"a","to":"b","step":1}]}"#);
        let scene = good.scene().expect("reads");
        assert_eq!((scene.nodes.len(), scene.edges.len()), (2, 1));
        assert!(raw(r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"icon":"emoji"}],"edges":[]}"#).scene().is_none());
        assert!(raw(r#"{"nodes":[{"id":"a","kind":"group","x":0,"y":0}],"edges":[]}"#).scene().is_none());
        assert!(raw(r#"{"nodes":[{"id":"a","kind":"mock","x":0,"y":0,"w":80}],"edges":[]}"#).scene().is_none());
        assert!(raw(r#"{"nodes":[{"id":"a","kind":"mock","x":0,"y":0,"w":80,"html":"<b>1</b>"}],"edges":[]}"#).scene().is_some());
        assert!(raw(r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0}],"edges":[{"id":"e","from":"a","to":"z"}]}"#).scene().is_none());
        assert!(raw(r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0},{"id":"a","kind":"box","x":1,"y":1}],"edges":[]}"#).scene().is_none());
    }
}
