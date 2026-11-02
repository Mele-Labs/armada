//! The `mock` nodes of a checked scene, read and written by Fleet when it keeps their styles. The
//! scene is JSON text on the ledger, and this crate is where bytes become values, so Fleet reaches
//! it only through these.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{too_heavy, RawScene, Sketch, MOST_SCENE_BYTES};

/// What Fleet needs of a `mock` node to choose its styles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockNode {
    pub id: String,
    /// The repository paths the agent named. Empty where it named none.
    pub styles: Vec<String>,
    /// The node carries some `css` of its own.
    pub has_css: bool,
}

fn mocks_of(scene: &Value) -> Vec<MockNode> {
    let nodes = scene.get("nodes").and_then(Value::as_array).into_iter().flatten();
    nodes
        .filter(|node| node.get("kind").and_then(Value::as_str) == Some("mock"))
        .filter_map(|node| {
            let id = node.get("id")?.as_str()?.to_string();
            let styles = node
                .get("styles")
                .and_then(Value::as_array)
                .map(|paths| paths.iter().filter_map(|path| path.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let has_css = node.get("css").and_then(Value::as_str).is_some_and(|css| !css.trim().is_empty());
            Some(MockNode { id, styles, has_css })
        })
        .collect()
}

impl Sketch {
    /// Its `mock` nodes, in scene order.
    pub fn mocks(&self) -> Vec<MockNode> {
        match crate::decode::<Value>("a kept sketch scene", self.scene.as_bytes()) {
            Ok(scene) => mocks_of(&scene),
            Err(_) => Vec::new(),
        }
    }

    /// This sketch with `kept` written on each node `hashes` names (node id to style set hash).
    /// **Not checked again**: `kept` is the one field the scene check refuses from an agent, and
    /// Fleet writes it after the check. It counts toward [`MOST_SCENE_BYTES`], so the cap is.
    pub fn with_kept(mut self, hashes: &BTreeMap<String, String>) -> Result<Sketch, String> {
        if hashes.is_empty() {
            return Ok(self);
        }
        let mut scene = crate::decode::<Value>("a kept sketch scene", self.scene.as_bytes()).map_err(|why| why.why)?;
        if let Some(Value::Array(nodes)) = scene.get_mut("nodes") {
            for node in nodes {
                let hash = node.get("id").and_then(Value::as_str).and_then(|id| hashes.get(id));
                if let (Some(hash), Some(fields)) = (hash, node.as_object_mut()) {
                    fields.insert(String::from("kept"), Value::String(hash.clone()));
                }
            }
        }
        let written = crate::encode(&scene).map_err(|why| why.to_string())?;
        let weight = written.len() + self.against.as_ref().map_or(0, String::len);
        if weight > MOST_SCENE_BYTES {
            return Err(format!(
                "{} Fleet adds a 32 character `kept` to each `mock` it draws with the app's styles",
                too_heavy(weight)
            ));
        }
        self.scene = written;
        Ok(self)
    }
}

/// The `mock` nodes of a scene a page embeds, as text. Empty where the text is not a scene: the page
/// is told that separately by `page_scene_problems`.
pub fn page_mocks(text: &str) -> Vec<MockNode> {
    match crate::decode::<RawScene>("sketch scene", text.trim().as_bytes()) {
        Ok(raw) => mocks_of(&raw.0),
        Err(_) => Vec::new(),
    }
}
