//! `show_sketch`: a session draws the person a diagram.
//!
//! **The scene is checked here, in the crate where bytes enter, against the rules of
//! `parseScene` in `packages/components/src/compositions/SketchScene/scene.ts`.** Bridge draws a
//! scene that does not validate as an empty frame and says nothing, so the only place an agent can
//! be told what is wrong with its drawing is before it is kept. `tests/sketch.rs` reads the three
//! lists below out of that file and fails when they differ.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::SessionId;

pub(crate) mod screen;

/// What a node may be.
pub const SCENE_KINDS: [&str; 7] = ["box", "group", "lane", "label", "code", "wire", "screen"];

/// What a `wire` node stands for.
pub const WIRE_PARTS: [&str; 6] = ["button", "input", "list", "card", "nav", "text"];

/// The outline a `box` node's `shape` may name.
pub const SCENE_SHAPES: [&str; 6] = ["box", "pill", "circle", "diamond", "cylinder", "hexagon"];

/// The colours a node's or an edge's `color` may name.
pub const SCENE_COLORS: [&str; 7] = ["blue", "teal", "green", "amber", "orange", "red", "grey"];

/// The registry glyphs a node's `icon` may name.
pub const SCENE_ICONS: [&str; 25] = [
    "activity", "bot", "box", "clock", "cpu", "eye", "file", "git-branch", "git-merge", "globe",
    "hammer", "hard-drive", "layers", "lock", "package", "rocket", "scale", "scroll-text",
    "server", "settings", "shield-check", "terminal", "webhook", "wrench", "zap",
];

/// Most nodes, edges and strokes one scene holds.
pub const MOST_NODES: usize = 80;
pub const MOST_EDGES: usize = 120;
pub const MOST_STROKES: usize = 20;
/// Most the scene weighs once written. **It rides on the session's record**, which is published
/// whole on every change, so a sketch is paid for by every event that follows it.
pub const MOST_SCENE_BYTES: usize = 32 * 1024;
/// Most characters in a sketch's title, which is also what a second call replaces it by.
pub const MOST_TITLE_CHARS: usize = 120;

const TOP_FIELDS: [&str; 3] = ["nodes", "edges", "strokes"];
pub(crate) const NODE_FIELDS: [&str; 15] =
    ["id", "kind", "x", "y", "w", "h", "title", "body", "icon", "lang", "wire", "screen", "step", "shape", "color"];
pub(crate) const EDGE_FIELDS: [&str; 7] = ["id", "from", "to", "label", "flow", "step", "color"];
const STROKE_FIELDS: [&str; 2] = ["id", "points"];
const POINT_FIELDS: [&str; 2] = ["x", "y"];

/// Problems listed in one refusal, so an agent fixes a drawing in a round or two and the answer
/// stays short.
const MOST_PROBLEMS_SAID: usize = 12;

/// A scene as the agent sent it. **Opaque to every crate but this one**: it is read only through
/// [`ShowSketch::checked`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RawScene(Value);

/// `show_sketch`: a session draws the person a diagram, which lands on its ledger.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowSketch {
    /// What the ledger calls it. **A second sketch with the same title replaces the first.**
    pub title: String,
    pub scene: RawScene,
    /// Which session draws it. **Read only where the connection places no hosted session**, as
    /// `ShowWindow`'s is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// The title of an earlier sketch of this session, which the owner reads this one as a change
    /// to. Fleet keeps that scene whole at the call, so a later replacement does not move the diff.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub against: Option<String>,
}

/// A sketch that passed every rule, in the form the ledger keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sketch {
    pub title: String,
    /// The scene as JSON text, because an attachment's detail holds strings only.
    pub scene: String,
    pub nodes: usize,
    pub edges: usize,
    /// The earlier scene it is read against, as `{"title", "scene"}` written out, with `scene` the
    /// earlier scene's JSON text. Absent where the call named none.
    pub against: Option<String>,
}

impl Sketch {
    /// This sketch read against `earlier`, the scene text of the sketch titled `title`. **The two
    /// are counted together in [`MOST_SCENE_BYTES`]**, because both ride on the record.
    pub fn read_against(mut self, title: &str, earlier: &str) -> Result<Sketch, String> {
        let kept = crate::encode(&serde_json::json!({ "title": title, "scene": earlier }))
            .map_err(|why| refusal(&[why.to_string()]))?;
        let weight = self.scene.len() + kept.len();
        if weight > MOST_SCENE_BYTES {
            return Err(refusal(&[format!(
                "the scene and the sketch `{title}` it is drawn against weigh {weight} bytes together \
                 and a sketch holds at most {MOST_SCENE_BYTES}. Draw less per sketch, or leave \
                 `against` out"
            )]));
        }
        self.against = Some(kept);
        Ok(self)
    }
}

impl ShowSketch {
    /// The title of the sketch this one is read against, trimmed. A blank one is refused rather than
    /// taken for none, because the agent meant something.
    pub fn against_title(&self) -> Result<Option<String>, String> {
        match self.against.as_deref().map(str::trim) {
            None => Ok(None),
            Some("") => Err(refusal(&[String::from("`against` is blank. Name the sketch to read this one against, or leave it out")])),
            Some(title) => Ok(Some(title.to_string())),
        }
    }

    /// The sketch, or every reason it is refused in one message an agent can act on.
    pub fn checked(&self) -> Result<Sketch, String> {
        self.checked_for("show_sketch")
    }

    /// [`Self::checked`] for a scene that arrived through another tool, whose name the refusal asks
    /// the agent to call again.
    pub fn checked_for(&self, tool: &str) -> Result<Sketch, String> {
        let mut problems = Vec::new();
        let title = self.title.trim().to_string();
        if title.is_empty() {
            problems.push(String::from("`title` is blank, and a sketch is replaced by its title"));
        } else if title.chars().count() > MOST_TITLE_CHARS {
            problems.push(format!("`title` is longer than {MOST_TITLE_CHARS} characters"));
        }
        let counts = scene_problems(&self.scene.0, &mut problems);
        if !problems.is_empty() {
            return Err(refusal_for(&problems, tool));
        }
        let (nodes, edges) = counts.unwrap_or_default();
        let scene = crate::encode(&self.scene.0).map_err(|why| refusal_for(&[why.to_string()], tool))?;
        if scene.len() > MOST_SCENE_BYTES {
            return Err(refusal_for(&[too_heavy(scene.len())], tool));
        }
        Ok(Sketch { title, scene, nodes, edges, against: None })
    }
}

fn too_heavy(bytes: usize) -> String {
    format!(
        "the scene is {bytes} bytes and a sketch holds at most {MOST_SCENE_BYTES}. Draw less \
         per sketch, or split it into two with their own titles"
    )
}

/// Every reason the scene text of a sketch a Job's document embeds will not draw, in the words
/// `show_sketch` refuses with and by the same rules; empty where it draws. **The page keeps a bad
/// scene and Bridge draws it as an empty frame**, so this is how the Job hears of it.
pub fn page_scene_problems(text: &str) -> Vec<String> {
    let raw = match crate::decode::<RawScene>("sketch scene", text.trim().as_bytes()) {
        Ok(raw) => raw,
        Err(why) => return vec![format!("the scene is not JSON: {}", why.why)],
    };
    let mut problems = Vec::new();
    scene_problems(&raw.0, &mut problems);
    if problems.is_empty() {
        if let Ok(written) = crate::encode(&raw.0) {
            if written.len() > MOST_SCENE_BYTES {
                problems.push(too_heavy(written.len()));
            }
        }
    }
    problems.truncate(MOST_PROBLEMS_SAID);
    problems
}

fn refusal(problems: &[String]) -> String {
    refusal_for(problems, "show_sketch")
}

fn refusal_for(problems: &[String], tool: &str) -> String {
    let said: Vec<&str> = problems.iter().take(MOST_PROBLEMS_SAID).map(String::as_str).collect();
    let more = match problems.len().saturating_sub(MOST_PROBLEMS_SAID) {
        0 => String::new(),
        left => format!("; and {left} more"),
    };
    format!("the sketch was not drawn: {}{more}. Fix these and call {tool} again", said.join("; "))
}

/// `(nodes, edges)` of a scene whose shape could be read, with what is wrong with it pushed.
fn scene_problems(scene: &Value, problems: &mut Vec<String>) -> Option<(usize, usize)> {
    let fields = match scene {
        Value::Object(fields) => fields,
        Value::String(_) => {
            problems.push(String::from(
                "`scene` is a string. Send the object itself with `nodes` and `edges`, not JSON text",
            ));
            return None;
        }
        other => {
            problems.push(format!("`scene` is {}. It must be an object with `nodes` and `edges`", kind_of(other)));
            return None;
        }
    };
    unknown(fields.keys(), &TOP_FIELDS, "`scene`", problems);
    let (Some(nodes), Some(edges)) = (
        listed(fields.get("nodes"), "nodes", problems),
        listed(fields.get("edges"), "edges", problems),
    ) else {
        return None;
    };
    if nodes.is_empty() {
        problems.push(String::from("`nodes` is empty, and a scene with nothing in it draws an empty frame"));
    }
    for (what, held, most) in [("nodes", nodes.len(), MOST_NODES), ("edges", edges.len(), MOST_EDGES)] {
        if held > most {
            problems.push(format!("`{what}` holds {held} and a sketch holds at most {most}. Split it into two sketches"));
            return None;
        }
    }
    let ids = node_problems(nodes, problems);
    edge_problems(edges, &ids, problems);
    if let Some(strokes) = fields.get("strokes") {
        stroke_problems(strokes, problems);
    }
    Some((nodes.len(), edges.len()))
}

fn listed<'a>(value: Option<&'a Value>, name: &str, problems: &mut Vec<String>) -> Option<&'a Vec<Value>> {
    match value {
        Some(Value::Array(list)) => Some(list),
        Some(other) => {
            problems.push(format!("`{name}` is {}. It must be a list", kind_of(other)));
            None
        }
        None => {
            problems.push(format!("`{name}` is missing. Send it as a list, empty where there is none"));
            None
        }
    }
}

/// Every node's id that is usable, for the edges to join.
fn node_problems(nodes: &[Value], problems: &mut Vec<String>) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for (at, node) in nodes.iter().enumerate() {
        let here = place("nodes", at, node);
        let Value::Object(fields) = node else {
            problems.push(format!("{here}: must be an object, is {}", kind_of(node)));
            continue;
        };
        unknown(fields.keys(), &NODE_FIELDS, &here, problems);
        match fields.get("id") {
            Some(Value::String(id)) if !id.is_empty() => match ids.iter().position(|one| one == id) {
                Some(first) => problems.push(format!("{here}: repeats the id of nodes[{first}]. Ids are unique")),
                None => ids.push(id.clone()),
            },
            _ => problems.push(format!("{here}: needs `id`, a non-empty string")),
        }
        let kind = match fields.get("kind") {
            Some(Value::String(kind)) if SCENE_KINDS.contains(&kind.as_str()) => Some(kind.as_str()),
            other => {
                problems.push(format!("{here}: `kind` is {}. Allowed: {}", seen(other), SCENE_KINDS.join(", ")));
                None
            }
        };
        for name in ["x", "y"] {
            number(fields.get(name), name, true, &here, problems);
        }
        let sized = matches!(kind, Some("group" | "lane"));
        for name in ["w", "h"] {
            if sized && fields.get(name).is_none() {
                problems.push(format!("{here}: a `{}` needs `{name}`, its size", kind.unwrap_or_default()));
            } else {
                number(fields.get(name), name, false, &here, problems);
            }
        }
        number(fields.get("step"), "step", false, &here, problems);
        for name in ["title", "body", "lang"] {
            text(fields.get(name), name, &here, problems);
        }
        one_of(fields.get("icon"), "icon", &SCENE_ICONS, &here, problems);
        one_of(fields.get("wire"), "wire", &WIRE_PARTS, &here, problems);
        one_of(fields.get("shape"), "shape", &SCENE_SHAPES, &here, problems);
        one_of(fields.get("color"), "color", &SCENE_COLORS, &here, problems);
        match (kind, fields.get("screen")) {
            (Some("screen"), Some(tree)) => screen::problems(tree, &format!("{here} screen"), problems),
            (Some("screen"), None) => problems.push(format!("{here}: a `screen` needs `screen`, its layout tree")),
            (Some(other), Some(_)) if other != "screen" => {
                problems.push(format!("{here}: `screen` is for a `screen` node, and this is a `{other}`"));
            }
            _ => {}
        }
    }
    ids
}

fn edge_problems(edges: &[Value], nodes: &[String], problems: &mut Vec<String>) {
    let mut ids: Vec<&str> = Vec::new();
    for (at, edge) in edges.iter().enumerate() {
        let here = place("edges", at, edge);
        let Value::Object(fields) = edge else {
            problems.push(format!("{here}: must be an object, is {}", kind_of(edge)));
            continue;
        };
        unknown(fields.keys(), &EDGE_FIELDS, &here, problems);
        match fields.get("id") {
            Some(Value::String(id)) if !id.is_empty() => match ids.iter().position(|one| one == id) {
                Some(first) => problems.push(format!("{here}: repeats the id of edges[{first}]. Ids are unique")),
                None => ids.push(id),
            },
            _ => problems.push(format!("{here}: needs `id`, a non-empty string")),
        }
        for end in ["from", "to"] {
            match fields.get(end) {
                Some(Value::String(named)) if nodes.contains(named) => {}
                Some(Value::String(named)) => problems.push(format!(
                    "{here}: `{end}` is `{named}`, which no node has. Node ids: {}",
                    nodes.iter().take(20).cloned().collect::<Vec<_>>().join(", ")
                )),
                other => problems.push(format!("{here}: `{end}` is {}. It must be the id of a node", seen(other))),
            }
        }
        text(fields.get("label"), "label", &here, problems);
        number(fields.get("step"), "step", false, &here, problems);
        one_of(fields.get("color"), "color", &SCENE_COLORS, &here, problems);
        if let Some(flow) = fields.get("flow") {
            if !flow.is_boolean() {
                problems.push(format!("{here}: `flow` is {}. It must be true or false", seen(Some(flow))));
            }
        }
    }
}

fn stroke_problems(strokes: &Value, problems: &mut Vec<String>) {
    let Value::Array(list) = strokes else {
        problems.push(format!("`strokes` is {}. It must be a list", kind_of(strokes)));
        return;
    };
    if list.len() > MOST_STROKES {
        problems.push(format!("`strokes` holds {} and a sketch holds at most {MOST_STROKES}", list.len()));
        return;
    }
    for (at, stroke) in list.iter().enumerate() {
        let here = format!("strokes[{at}]");
        let Value::Object(fields) = stroke else {
            problems.push(format!("{here}: must be an object, is {}", kind_of(stroke)));
            continue;
        };
        unknown(fields.keys(), &STROKE_FIELDS, &here, problems);
        if !matches!(fields.get("id"), Some(Value::String(_))) {
            problems.push(format!("{here}: needs `id`, a string"));
        }
        let Some(Value::Array(points)) = fields.get("points") else {
            problems.push(format!("{here}: needs `points`, a list of objects with `x` and `y`"));
            continue;
        };
        for (index, point) in points.iter().enumerate() {
            let spot = format!("{here}.points[{index}]");
            let Value::Object(coordinates) = point else {
                problems.push(format!("{spot}: must be an object with `x` and `y`, is {}", kind_of(point)));
                break;
            };
            unknown(coordinates.keys(), &POINT_FIELDS, &spot, problems);
            for name in POINT_FIELDS {
                number(coordinates.get(name), name, true, &spot, problems);
            }
        }
    }
}

/// `nodes[2] `api``: where a part is, and what it calls itself where it has an id.
fn place(list: &str, at: usize, part: &Value) -> String {
    match part.get("id") {
        Some(Value::String(id)) if !id.is_empty() => format!("{list}[{at}] `{id}`"),
        _ => format!("{list}[{at}]"),
    }
}

fn unknown<'a>(
    keys: impl Iterator<Item = &'a String>,
    allowed: &[&str],
    here: &str,
    problems: &mut Vec<String>,
) {
    let strange: Vec<String> =
        keys.filter(|key| !allowed.contains(&key.as_str())).map(|key| format!("`{key}`")).collect();
    if !strange.is_empty() {
        problems.push(format!(
            "{here}: has {}, which it does not take. Fields: {}",
            strange.join(", "),
            allowed.join(", ")
        ));
    }
}

fn number(value: Option<&Value>, name: &str, required: bool, here: &str, problems: &mut Vec<String>) {
    match value {
        Some(Value::Number(_)) => {}
        None if !required => {}
        None => problems.push(format!("{here}: needs `{name}`, a number")),
        other => problems.push(format!("{here}: `{name}` is {}. It must be a number", seen(other))),
    }
}

fn text(value: Option<&Value>, name: &str, here: &str, problems: &mut Vec<String>) {
    match value {
        None | Some(Value::String(_)) => {}
        other => problems.push(format!("{here}: `{name}` is {}. It must be a string", seen(other))),
    }
}

fn one_of(value: Option<&Value>, name: &str, allowed: &[&str], here: &str, problems: &mut Vec<String>) {
    match value {
        None => {}
        Some(Value::String(said)) if allowed.contains(&said.as_str()) => {}
        other => problems.push(format!("{here}: `{name}` is {}. Allowed: {}", seen(other), allowed.join(", "))),
    }
}

/// A value as a refusal quotes it: a string by its text, anything else by its kind.
fn seen(value: Option<&Value>) -> String {
    match value {
        None => String::from("missing"),
        Some(Value::String(said)) => format!("`{}`", said.chars().take(40).collect::<String>()),
        Some(other) => kind_of(other),
    }
}

fn kind_of(value: &Value) -> String {
    String::from(match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    })
}
