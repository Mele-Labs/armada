//! Where each part of a level stands, and the curves that join them, worked out in pixels from one `SystemMap`. Nothing here
//! touches the screen: the graph draws what this returns, and the keys and the hover read it.
//!
//! Level one is People, then the project's boundary with its entry points, what they reach and its stores, then Outside. Level
//! two and three stand in lanes by the part each piece belongs to, with stores and outside last. Ports are spread along a side and
//! sorted by the far end, so curves that leave one part do not cross where they need not.

use std::collections::{HashMap, HashSet};

use ipc::{SystemMap, SystemPart, SystemPartKind};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Pt {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn centre(&self) -> Pt {
        Pt { x: self.x + self.w / 2., y: self.y + self.h / 2. }
    }
}

/// The sizes the arrangement is made of, which the view reads from the tokens.
#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    /// A part with its name and a line under it.
    pub node: f32,
    /// A part with its name alone.
    pub node_plain: f32,
    /// One file under a part's name, on level three.
    pub file: f32,
    /// The least room between two parts one above the other.
    pub gap: f32,
    /// The room a lane keeps at its top for its name.
    pub lane_head: f32,
    pub lane_pad: f32,
    pub lane_gap: f32,
    /// Between two columns inside the boundary.
    pub col_gap: f32,
    pub bottom: f32,
    /// What a character of a link's label is wide, to leave room for the label in the gap it sits in.
    pub char: f32,
    /// The least a part's column is wide, whatever it holds.
    pub column: f32,
    /// What a character of a part's name is wide, and what a part's tile, padding and arrow take beside its text: together they
    /// give the width a part needs to show its name and its path whole.
    pub name_char: f32,
    pub chrome: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Metrics { node: 52., node_plain: 42., file: 15., gap: 16., lane_head: 30., lane_pad: 20., lane_gap: 40., col_gap: 64., bottom: 20., char: 7., column: 170., name_char: 8., chrome: 76. }
    }
}

impl Metrics {
    /// Every size times `k`, for a graph drawn smaller to fit its room.
    pub fn scaled(&self, k: f32) -> Metrics {
        Metrics {
            node: self.node * k,
            node_plain: self.node_plain * k,
            file: self.file * k,
            gap: self.gap * k,
            lane_head: self.lane_head * k,
            lane_pad: self.lane_pad * k,
            lane_gap: self.lane_gap * k,
            col_gap: self.col_gap * k,
            bottom: self.bottom * k,
            char: self.char * k,
            column: self.column * k,
            name_char: self.name_char * k,
            chrome: self.chrome * k,
        }
    }
}

/// The least the graph is drawn at before it scrolls across instead.
pub const FLOOR: f32 = 0.75;

/// The longest path a part is given room to show whole, in characters; a longer one is cut with an ellipsis.
const META_LONGEST: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaneKind {
    /// A band with no edge of its own: People, Outside, and the stores.
    Band,
    /// The project, or a part, drawn with a dashed outline round what is inside it.
    Boundary,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lane {
    pub rect: Rect,
    pub title: String,
    pub kind: LaneKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub kind: SystemPartKind,
    /// The part a piece belongs to.
    pub within: Option<String>,
    /// The line under the name: where its code is.
    pub meta: Option<String>,
    /// The files a feature runs through in it, on level three.
    pub files: Vec<String>,
    pub rect: Rect,
    pub zooms: bool,
    /// The librarian added it and he has not kept it.
    pub librarian: bool,
}

/// The least a curve leaves its part straight before it bends.
const PULL_LEAST: f32 = 36.;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    fn normal(self) -> Pt {
        match self {
            Side::Left => Pt { x: -1., y: 0. },
            Side::Right => Pt { x: 1., y: 0. },
            Side::Top => Pt { x: 0., y: -1. },
            Side::Bottom => Pt { x: 0., y: 1. },
        }
    }

    fn vertical(self) -> bool {
        matches!(self, Side::Top | Side::Bottom)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub label: Option<String>,
    pub step: Option<u32>,
    pub a: Pt,
    pub b: Pt,
    /// The curve from `a` to `b` as cubic pieces end to end, each its start, two controls and its end: one piece between two lanes,
    /// several where it runs out to its lane's gutter and back.
    pub path: Vec<[Pt; 4]>,
    /// Both ends stand in one lane of level two or three, so it runs by the lane's gutter and is drawn only as `shown` says.
    pub lane: bool,
    /// Where its pill stands: the curve's middle, or in the gutter for a link inside a lane.
    pub mid: Pt,
}

impl Edge {
    /// A point of the curve, `t` from 0 at the source to 1 at the target, each piece taking an equal share of `t`.
    pub fn at(&self, t: f32) -> Pt {
        let Some(last) = self.path.len().checked_sub(1) else { return self.a };
        let along = t.clamp(0., 1.) * self.path.len() as f32;
        let piece = (along.floor() as usize).min(last);
        let [a, c1, c2, b] = self.path[piece];
        let t = along - piece as f32;
        let u = 1. - t;
        let (w0, w1, w2, w3) = (u * u * u, 3. * u * u * t, 3. * u * t * t, t * t * t);
        Pt { x: w0 * a.x + w1 * c1.x + w2 * c2.x + w3 * b.x, y: w0 * a.y + w1 * c1.y + w2 * c2.y + w3 * b.y }
    }

    /// The way the curve runs where it meets its target, a unit vector.
    pub fn heading(&self) -> Pt {
        let [_, _, c2, b] = self.path.last().copied().unwrap_or([self.a, self.a, self.a, self.b]);
        let (dx, dy) = (b.x - c2.x, b.y - c2.y);
        let long = dx.hypot(dy).max(0.001);
        Pt { x: dx / long, y: dy / long }
    }
}

/// Where each lane's sub-columns stand on level two and three, so a link between two parts of one lane runs in the room beside
/// them and never over a part.
struct Gutters {
    /// Each part's lane and sub-column.
    place: Vec<(usize, usize)>,
    /// Each lane's left and right edge, and its sub-columns' left and right edges, left to right.
    lanes: Vec<(f32, f32, Vec<(f32, f32)>)>,
    /// A level above every part and one below, clear across every lane, for a link that runs past a sub-column.
    over: f32,
    under: f32,
}

impl Gutters {
    /// Where the run beside sub-column `col` of `lane` goes on `side`, and how far that is from the column's edge.
    fn beside(&self, lane: usize, col: usize, side: Side) -> (f32, f32) {
        let (left, right, cols) = &self.lanes[lane];
        let (x0, x1) = cols[col];
        let (near, far) = match side {
            Side::Left => (x0, if col == 0 { *left } else { cols[col - 1].1 }),
            _ => (x1, cols.get(col + 1).map_or(*right, |next| next.0)),
        };
        ((near + far) / 2., (far - near).abs() / 2.)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Arranged {
    pub lanes: Vec<Lane>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub width: f32,
    pub height: f32,
    /// How much smaller than its natural size it is drawn, 1 where it fits the room.
    pub scale: f32,
}

impl Arranged {
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|node| node.id == id)
    }
}

/// What the pointer or the keys are on, which lights what it talks to.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Hover {
    #[default]
    None,
    Part(String),
    Feature(String),
}

/// The parts and links drawn bright while the rest dims.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lit {
    pub nodes: HashSet<usize>,
    pub edges: HashSet<usize>,
}

/// Where a part stands among its lane's columns.
fn column_of(part: &SystemPart, deep: bool) -> (usize, usize) {
    if deep {
        return (0, 0);
    }
    match part.kind {
        SystemPartKind::Person => (0, 0),
        SystemPartKind::Entry => (1, 0),
        SystemPartKind::Service | SystemPartKind::Piece => (1, 1),
        SystemPartKind::Store => (1, 2),
        SystemPartKind::Outside => (2, 0),
    }
}

/// The key of the lane a part of level two or three stands in.
fn lane_key(part: &SystemPart) -> String {
    match part.kind {
        SystemPartKind::Store | SystemPartKind::Outside => "~stores".to_string(),
        _ => part.within.clone().unwrap_or_else(|| "~self".to_string()),
    }
}

fn node_height(part: &SystemPart, files: usize, metrics: &Metrics) -> f32 {
    if files > 0 {
        return metrics.node_plain + metrics.file * files as f32 + metrics.gap;
    }
    if part.paths.is_empty() && !matches!(part.origin, ipc::PartOrigin::Librarian { .. }) {
        metrics.node_plain
    } else {
        metrics.node
    }
}

/// The first path of a part, as its line under the name.
fn meta_of(part: &SystemPart) -> Option<String> {
    part.paths.first().map(|path| path.trim_end_matches('/').to_string()).filter(|path| !path.is_empty())
}

/// The order lanes stand in on level two and three: a lane whose parts reach the others stands first.
fn lane_order(keys: &[String], map: &SystemMap, key_of: &HashMap<&str, usize>) -> Vec<usize> {
    let mut depth = vec![0usize; keys.len()];
    // A link from one lane to another pushes the other one place right, for as many rounds as there are lanes.
    for _ in 0..keys.len() {
        for link in &map.links {
            let (Some(&from), Some(&to)) = (key_of.get(link.from.as_str()), key_of.get(link.to.as_str())) else { continue };
            if from != to && depth[to] <= depth[from] && depth[from] < keys.len() {
                depth[to] = depth[from] + 1;
            }
        }
    }
    let stores = keys.iter().position(|key| key == "~stores");
    let mut order: Vec<usize> = (0..keys.len()).collect();
    order.sort_by_key(|&at| (Some(at) == stores, depth[at], at));
    order
}

/// Stand every part of `map` in `width` by at least `min_height`, and join them. `depth` is 1 for the project, 2 for a part and
/// 3 for a feature; `title_of` names the part a lane of level two or three is for.
pub fn arrange(map: &SystemMap, depth: usize, project: &str, title_of: &dyn Fn(&str) -> Option<String>, metrics: &Metrics, width: f32, min_height: f32) -> Arranged {
    // Parts wrap into as many sub-columns as the room's height asks for; where those would take more than the room's width, the lane
    // keeps fewer, one fewer at a time, and grows taller instead, which the block scrolls down.
    let mut across = usize::MAX;
    loop {
        let (arranged, widest) = lay(map, depth, project, title_of, metrics, width, min_height, across);
        if arranged.width <= width + 1. || width <= 0. || widest <= 1 {
            return arranged;
        }
        across = widest - 1;
    }
}

/// `arrange` with no lane wider than `across` sub-columns; also how many sub-columns the widest lane took.
#[allow(clippy::too_many_arguments)]
fn lay(map: &SystemMap, depth: usize, project: &str, title_of: &dyn Fn(&str) -> Option<String>, metrics: &Metrics, width: f32, min_height: f32, across: usize) -> (Arranged, usize) {
    let deep = depth > 1;
    let parts = &map.parts;
    if parts.is_empty() {
        return (Arranged { width, height: min_height, scale: 1., ..Arranged::default() }, 1);
    }
    let files_of = |part: &SystemPart| -> Vec<String> { map.stops.iter().filter(|stop| stop.part == part.id).flat_map(|stop| stop.files.clone()).collect() };
    let nodes_files: Vec<Vec<String>> = parts.iter().map(|part| if depth == 3 { files_of(part) } else { Vec::new() }).collect();
    let heights: Vec<f32> = parts.iter().zip(&nodes_files).map(|(part, files)| node_height(part, files.len(), metrics)).collect();
    let index: HashMap<&str, usize> = parts.iter().enumerate().map(|(at, part)| (part.id.as_str(), at)).collect();

    // ---- lanes and columns: every part gets a lane and a column of its lane --------------------------------------------------
    let mut lane_titles: Vec<(String, LaneKind)> = Vec::new();
    let mut lane_of: Vec<(usize, usize)> = Vec::new();
    if deep {
        let mut keys: Vec<String> = Vec::new();
        for part in parts {
            let key = lane_key(part);
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        let key_of: HashMap<&str, usize> = parts.iter().map(|part| (part.id.as_str(), keys.iter().position(|key| *key == lane_key(part)).unwrap_or(0))).collect();
        let order = lane_order(&keys, map, &key_of);
        let place: HashMap<usize, usize> = order.iter().enumerate().map(|(at, key)| (*key, at)).collect();
        for &key in &order {
            let title = match keys[key].as_str() {
                "~stores" => {
                    let stores = parts.iter().any(|part| part.kind == SystemPartKind::Store);
                    let outside = parts.iter().any(|part| part.kind == SystemPartKind::Outside);
                    match (stores, outside) {
                        (true, true) => "Stores and outside",
                        (true, false) => "Stores",
                        _ => "Outside",
                    }
                    .to_string()
                }
                "~self" => map.part.as_deref().and_then(title_of).unwrap_or_else(|| project.to_string()),
                other => title_of(other).unwrap_or_else(|| other.rsplit('/').next().unwrap_or(other).to_string()),
            };
            let band = keys[key] == "~stores";
            lane_titles.push((title, if band { LaneKind::Band } else { LaneKind::Boundary }));
        }
        for part in parts {
            let key = keys.iter().position(|key| *key == lane_key(part)).unwrap_or(0);
            lane_of.push((place[&key], 0));
        }
    } else {
        let present = |lane: usize| parts.iter().any(|part| column_of(part, false).0 == lane);
        let mut lanes_present: Vec<usize> = Vec::new();
        for (lane, title, kind) in [(0, "People", LaneKind::Band), (1, project, LaneKind::Boundary), (2, "Outside", LaneKind::Band)] {
            if present(lane) {
                lanes_present.push(lane);
                lane_titles.push((title.to_string(), kind));
            }
        }
        // The boundary's columns that hold a part, in order, so an empty one takes no room.
        let mut inner: Vec<usize> = parts.iter().map(|part| column_of(part, false)).filter(|(lane, _)| *lane == 1).map(|(_, col)| col).collect();
        inner.sort_unstable();
        inner.dedup();
        for part in parts {
            let (lane, col) = column_of(part, false);
            let at = lanes_present.iter().position(|one| *one == lane).unwrap_or(0);
            let col = if lane == 1 { inner.iter().position(|one| *one == col).unwrap_or(0) } else { 0 };
            lane_of.push((at, col));
        }
    }

    // A column that would run past the room's height wraps into sub-columns, a grid inside its lane.
    let room_tall = min_height - (metrics.lane_head + metrics.gap / 2.) - metrics.bottom;
    let widest = if min_height > 0. { wrap(&mut lane_of, &heights, room_tall, metrics.gap, across) } else { 1 };

    // ---- columns left to right, flattened ------------------------------------------------------------------------------
    let lane_count = lane_titles.len();
    let mut cols_in_lane = vec![0usize; lane_count];
    for &(lane, col) in &lane_of {
        cols_in_lane[lane] = cols_in_lane[lane].max(col + 1);
    }
    let first_col: Vec<usize> = (0..lane_count).map(|lane| cols_in_lane[..lane].iter().sum()).collect();
    let total_cols: usize = cols_in_lane.iter().sum();
    let col_of: Vec<usize> = lane_of.iter().map(|&(lane, col)| first_col[lane] + col).collect();

    // A person's and an outside part's column is narrower than a part's, since each holds a name and little else.
    let weight_of = |lane: usize| -> f32 {
        match (deep, lane_titles[lane].1, lane_titles[lane].0.as_str()) {
            (false, LaneKind::Band, _) => 0.8,
            (true, LaneKind::Band, _) => 0.9,
            _ => 1.,
        }
    };
    // The gap after a column, inside its lane: wide enough for the longest label of a link that crosses it.
    let mut gap_after = vec![0f32; total_cols];
    for lane in 0..lane_count {
        for col in 0..cols_in_lane[lane].saturating_sub(1) {
            let at = first_col[lane] + col;
            let longest = map
                .links
                .iter()
                .filter(|link| {
                    let (Some(&from), Some(&to)) = (index.get(link.from.as_str()), index.get(link.to.as_str())) else { return false };
                    (col_of[from], col_of[to]) == (at, at + 1) || (col_of[from], col_of[to]) == (at + 1, at)
                })
                .map(|link| link.label.as_deref().map_or(0, |label| label.chars().count()))
                .max()
                .unwrap_or(0);
            gap_after[at] = metrics.col_gap.max(longest as f32 * metrics.char + metrics.gap * 2.);
        }
    }
    // The gap after a lane: wide enough, with the lanes' own padding, for the longest label of a link between it and the next.
    let lane_gaps: Vec<f32> = (0..lane_count)
        .map(|lane| {
            let longest = map
                .links
                .iter()
                .filter(|link| {
                    let (Some(&from), Some(&to)) = (index.get(link.from.as_str()), index.get(link.to.as_str())) else { return false };
                    (lane_of[from].0, lane_of[to].0) == (lane, lane + 1) || (lane_of[from].0, lane_of[to].0) == (lane + 1, lane)
                })
                .map(|link| link.label.as_deref().map_or(0, |label| label.chars().count()))
                .max()
                .unwrap_or(0);
            metrics.lane_gap.max(longest as f32 * metrics.char + metrics.gap * 3. - metrics.lane_pad * 2.)
        })
        .collect();
    // What each column needs to show its parts' names and paths whole.
    let mut need = vec![metrics.column; total_cols];
    let col_lane: Vec<usize> = {
        let mut out = vec![0; total_cols];
        for (at, &(lane, _)) in lane_of.iter().enumerate() {
            out[col_of[at]] = lane;
        }
        out
    };
    for (at, part) in parts.iter().enumerate() {
        let meta = if nodes_files[at].is_empty() { meta_of(part).map_or(0, |meta| meta.chars().count().min(META_LONGEST)) } else { 0 };
        let file = nodes_files[at].iter().map(|file| file.chars().count()).max().unwrap_or(0);
        let wide = (part.name.chars().count() as f32 * metrics.name_char).max(meta.max(file) as f32 * metrics.char);
        let col = col_of[at];
        need[col] = need[col].max(wide + metrics.chrome);
    }
    let room = width - lane_gaps.iter().take(lane_count.saturating_sub(1)).sum::<f32>() - metrics.lane_pad * 2. * lane_count as f32 - gap_after.iter().sum::<f32>();
    // Columns share what room there is by weight; one that would be left narrower than it needs keeps what it needs, and the rest share the remainder.
    let col_weight: Vec<f32> = (0..total_cols).map(|col| weight_of(col_lane[col])).collect();
    let floor_of: Vec<f32> = (0..total_cols).map(|col| need[col].max(metrics.column * col_weight[col])).collect();
    let mut held = vec![false; total_cols];
    let unit = loop {
        let free: f32 = (0..total_cols).filter(|col| !held[*col]).map(|col| col_weight[col]).sum::<f32>().max(0.1);
        let kept: f32 = (0..total_cols).filter(|col| held[*col]).map(|col| floor_of[col]).sum();
        let unit = ((room - kept) / free).max(0.);
        let mut changed = false;
        for col in 0..total_cols {
            if !held[col] && col_weight[col] * unit < floor_of[col] {
                held[col] = true;
                changed = true;
            }
        }
        if !changed || held.iter().all(|one| *one) {
            break unit;
        }
    };

    let mut col_x = vec![0f32; total_cols];
    let mut col_w = vec![0f32; total_cols];
    let mut lane_rects: Vec<(f32, f32)> = Vec::new();
    let mut x = 0.;
    for lane in 0..lane_count {
        let start = x;
        x += metrics.lane_pad;
        for col in 0..cols_in_lane[lane] {
            let at = first_col[lane] + col;
            col_x[at] = x;
            col_w[at] = if held[at] { floor_of[at] } else { col_weight[at] * unit };
            x += col_w[at];
            x += gap_after[at];
        }
        x += metrics.lane_pad;
        lane_rects.push((start, x - start));
        x += lane_gaps[lane];
    }

    let extent = (x - lane_gaps.last().copied().unwrap_or(0.)).max(width);

    // ---- vertical order and place ---------------------------------------------------------------------------------------
    let mut stack: Vec<Vec<usize>> = vec![Vec::new(); total_cols];
    for (at, col) in col_of.iter().enumerate() {
        stack[*col].push(at);
    }
    let top = metrics.lane_head + metrics.gap / 2.;
    // What a column stands as tall as; where one is taller than the room the block scrolls, and the columns that fit stand from the
    // top of their lane, in view, and not level with whatever part of the tall one they link to.
    let tall_of = |col: &[usize]| col.iter().map(|&at| heights[at] + metrics.gap).sum::<f32>() - metrics.gap;
    let scrolls = min_height > 0. && stack.iter().any(|col| tall_of(col) > room_tall);
    let mut centre_y = vec![f32::NAN; parts.len()];
    let mut ys = vec![0f32; parts.len()];
    for col in 0..total_cols {
        // Parts that reach into this column from earlier ones stand level with what they reach, which keeps the curves short.
        let mut keyed: Vec<(f32, usize)> = stack[col]
            .iter()
            .enumerate()
            .map(|(rank, &at)| {
                let near: Vec<f32> = map
                    .links
                    .iter()
                    .filter_map(|link| {
                        let (from, to) = (index.get(link.from.as_str())?, index.get(link.to.as_str())?);
                        let other = if *to == at { *from } else if *from == at { *to } else { return None };
                        centre_y[other].is_finite().then_some(centre_y[other])
                    })
                    .collect();
                let key = if near.is_empty() { top + rank as f32 } else { near.iter().sum::<f32>() / near.len() as f32 };
                (key, at)
            })
            .collect();
        keyed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        // Each stands where it wants to, pushed down only as far as the one above it needs.
        let stood = tall_of(&stack[col]);
        let from_top = scrolls && stood <= room_tall;
        let mut floor = top;
        for &(key, at) in &keyed {
            let y = if from_top { floor } else { (key - heights[at] / 2.).max(floor) };
            ys[at] = y;
            centre_y[at] = y + heights[at] / 2.;
            floor = y + heights[at] + metrics.gap;
        }
        // Where the room's height is known, what was pushed down past its bottom is lifted back in, from the last one up.
        if min_height > 0. && stood <= room_tall {
            let mut ceiling = min_height - metrics.bottom;
            for &(_, at) in keyed.iter().rev() {
                ys[at] = ys[at].min(ceiling - heights[at]).max(top);
                centre_y[at] = ys[at] + heights[at] / 2.;
                ceiling = ys[at] - metrics.gap;
            }
        }
    }
    let height = (0..parts.len()).map(|at| ys[at] + heights[at]).fold(top, f32::max) + metrics.bottom;

    let nodes: Vec<Node> = parts
        .iter()
        .enumerate()
        .map(|(at, part)| Node {
            id: part.id.clone(),
            name: part.name.clone(),
            kind: part.kind,
            within: part.within.clone(),
            meta: if nodes_files[at].is_empty() { meta_of(part) } else { None },
            files: nodes_files[at].clone(),
            rect: Rect { x: col_x[col_of[at]], y: ys[at], w: col_w[col_of[at]], h: heights[at] },
            zooms: part.zooms,
            librarian: matches!(part.origin, ipc::PartOrigin::Librarian { .. }),
        })
        .collect();
    let lanes: Vec<Lane> = lane_rects.iter().zip(&lane_titles).map(|(&(x, w), (title, kind))| Lane { rect: Rect { x, y: 0., w, h: height }, title: title.clone(), kind: *kind }).collect();
    let gutters = deep.then(|| Gutters {
        place: lane_of.clone(),
        lanes: (0..lane_count)
            .map(|lane| {
                let (x, w) = lane_rects[lane];
                (x, x + w, (0..cols_in_lane[lane]).map(|col| first_col[lane] + col).map(|at| (col_x[at], col_x[at] + col_w[at])).collect())
            })
            .collect(),
        over: top - metrics.gap / 4.,
        under: height - metrics.bottom / 2.,
    });
    let edges = join(map, &index, &nodes, metrics, gutters.as_ref());
    (Arranged { lanes, nodes, edges, width: extent, height, scale: 1. }, widest)
}

/// Splits each column of each lane into sub-columns, filling one down to `cap` before the next, so a lane of many parts is a grid
/// as tall as the room and not a column as tall as the parts. A column that fits is left as it is. No column takes more than
/// `across` sub-columns, which makes them taller than `cap`; returns the most any column took.
fn wrap(lane_of: &mut [(usize, usize)], heights: &[f32], cap: f32, gap: f32, across: usize) -> usize {
    let lanes = lane_of.iter().map(|(lane, _)| *lane).max().map_or(0, |last| last + 1);
    let was = lane_of.to_vec();
    let mut most = 1;
    for lane in 0..lanes {
        let mut cols: Vec<usize> = was.iter().filter(|(one, _)| *one == lane).map(|(_, col)| *col).collect();
        cols.sort_unstable();
        cols.dedup();
        let mut offset = 0;
        for col in cols {
            let members: Vec<usize> = (0..was.len()).filter(|&at| was[at] == (lane, col)).collect();
            let fill = |limit: f32| -> Vec<usize> {
                let (mut used, mut sub) = (0., 0);
                members
                    .iter()
                    .map(|&at| {
                        if used > 0. && used + gap + heights[at] > limit {
                            sub += 1;
                            used = 0.;
                        }
                        used += if used > 0. { gap } else { 0. } + heights[at];
                        sub
                    })
                    .collect()
            };
            let mut subs = fill(cap);
            if subs.last().is_some_and(|last| last + 1 > across) {
                let total: f32 = members.iter().map(|&at| heights[at] + gap).sum();
                let tallest = members.iter().map(|&at| heights[at]).fold(0., f32::max);
                subs = fill(total / across as f32 + tallest);
            }
            let made = subs.last().map_or(0, |last| last + 1);
            for (&at, sub) in members.iter().zip(subs) {
                lane_of[at].1 = offset + sub;
            }
            offset += made;
            most = most.max(made);
        }
    }
    most
}

/// `arrange`, made smaller where the parts need more room than there is: every size shrinks together, to a floor of `FLOOR`, and
/// below that the graph keeps its size and the block scrolls across.
pub fn arrange_fitted(map: &SystemMap, depth: usize, project: &str, title_of: &dyn Fn(&str) -> Option<String>, metrics: &Metrics, width: f32, min_height: f32) -> Arranged {
    let natural = arrange(map, depth, project, title_of, metrics, width, min_height);
    if natural.width <= width + 1. || width <= 0. {
        return natural;
    }
    let mut scale = (width / natural.width).clamp(FLOOR, 1.);
    let mut smaller = arrange(map, depth, project, title_of, &metrics.scaled(scale), width, min_height);
    // Text does not shrink in step with the gaps by a hair, so one correction.
    if smaller.width > width + 1. && scale > FLOOR {
        scale = (scale * width / smaller.width).clamp(FLOOR, 1.);
        smaller = arrange(map, depth, project, title_of, &metrics.scaled(scale), width, min_height);
    }
    smaller.scale = scale;
    smaller
}

/// Each link as a curve between a port on one part's side and a port on the other's. A link between two parts of one lane leaves
/// its part sideways into the gutter beside its sub-column, runs along it, and comes back in, so it never crosses a part.
fn join(map: &SystemMap, index: &HashMap<&str, usize>, nodes: &[Node], metrics: &Metrics, gutters: Option<&Gutters>) -> Vec<Edge> {
    let links: Vec<(usize, usize, usize)> = map
        .links
        .iter()
        .enumerate()
        .filter_map(|(at, link)| {
            let (from, to) = (*index.get(link.from.as_str())?, *index.get(link.to.as_str())?);
            (from != to).then_some((at, from, to))
        })
        .collect();
    let inside = |from: usize, to: usize| gutters.filter(|gutters| gutters.place[from].0 == gutters.place[to].0);
    let mut sides: Vec<(Side, Side)> = links
        .iter()
        .map(|&(_, from, to)| {
            let (a, b) = (nodes[from].rect, nodes[to].rect);
            if b.x >= a.right() - 0.5 {
                (Side::Right, Side::Left)
            } else if b.right() <= a.x + 0.5 {
                (Side::Left, Side::Right)
            } else if b.y >= a.y {
                (Side::Bottom, Side::Top)
            } else {
                (Side::Top, Side::Bottom)
            }
        })
        .collect();
    // A link inside one sub-column goes out by a side its label does not spill off the block from, then the side its two parts'
    // links to other lanes leave free, so it does not tangle with the links drawn at rest, then the roomier gutter, then the right.
    let mut taken: HashMap<(usize, Side), usize> = HashMap::new();
    for (at, &(_, from, to)) in links.iter().enumerate() {
        if inside(from, to).is_none() {
            *taken.entry((from, sides[at].0)).or_default() += 1;
            *taken.entry((to, sides[at].1)).or_default() += 1;
        }
    }
    for (at, &(wire, from, to)) in links.iter().enumerate() {
        let Some(gutters) = inside(from, to) else { continue };
        let ((lane, here), (_, there)) = (gutters.place[from], gutters.place[to]);
        sides[at] = match here.cmp(&there) {
            std::cmp::Ordering::Less => (Side::Right, Side::Left),
            std::cmp::Ordering::Greater => (Side::Left, Side::Right),
            std::cmp::Ordering::Equal => {
                let busy = |side: Side| taken.get(&(from, side)).copied().unwrap_or(0) + taken.get(&(to, side)).copied().unwrap_or(0);
                let room = |side: Side| gutters.beside(lane, here, side).1;
                let link = &map.links[wire];
                let chars = link.label.as_deref().map_or(0, |label| label.chars().count()) + if link.step.is_some() { 2 } else { 0 };
                let wide = if chars == 0 { 0. } else { chars as f32 * metrics.char + metrics.gap * 2.25 };
                let (column, first, last) = (nodes[from].rect, gutters.lanes.first().map_or(0., |lane| lane.0), gutters.lanes.last().map_or(0., |lane| lane.1));
                let spills = |side: Side| if side == Side::Left { column.x - wide < first } else { column.right() + wide > last };
                let key = |side: Side| (spills(side), busy(side), -room(side));
                let left = key(Side::Left) < key(Side::Right);
                if left { (Side::Left, Side::Left) } else { (Side::Right, Side::Right) }
            }
        };
    }
    // Every end on a side of a part, with the place of the far end, so the ends can be sorted by it.
    let mut ends: HashMap<(usize, Side), Vec<(usize, bool, f32)>> = HashMap::new();
    for (at, &(_, from, to)) in links.iter().enumerate() {
        let far = |other: usize, side: Side| if side.vertical() { nodes[other].rect.centre().x } else { nodes[other].rect.centre().y };
        ends.entry((from, sides[at].0)).or_default().push((at, true, far(to, sides[at].0)));
        ends.entry((to, sides[at].1)).or_default().push((at, false, far(from, sides[at].1)));
    }
    let mut ports: HashMap<(usize, bool), Pt> = HashMap::new();
    for ((node, side), list) in ends.iter_mut() {
        list.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        let rect = nodes[*node].rect;
        let count = list.len();
        for (rank, &(link, source, _)) in list.iter().enumerate() {
            let share = if count == 1 { 0.5 } else { 0.22 + 0.56 * rank as f32 / (count - 1) as f32 };
            let at = match side {
                Side::Left => Pt { x: rect.x, y: rect.y + rect.h * share },
                Side::Right => Pt { x: rect.right(), y: rect.y + rect.h * share },
                Side::Top => Pt { x: rect.x + rect.w * share, y: rect.y },
                Side::Bottom => Pt { x: rect.x + rect.w * share, y: rect.bottom() },
            };
            ports.insert((link, source), at);
        }
    }
    // A link inside a lane keeps the run its pill stands on: its x, which way is away from the part, the room to the part's edge, and
    // where it starts and ends down the block.
    let mut runs: Vec<Option<(f32, f32, f32, f32, f32)>> = Vec::new();
    let mut edges = links
        .iter()
        .enumerate()
        .map(|(at, &(wire, from, to))| {
            let (a, b) = (ports[&(at, true)], ports[&(at, false)]);
            let (out, into) = (sides[at].0.normal(), sides[at].1.normal());
            let label = map.links[wire].label.clone().filter(|label| !label.is_empty());
            if let Some(gutters) = inside(from, to) {
                let ((lane, here), (_, there)) = (gutters.place[from], gutters.place[to]);
                let (xa, room) = gutters.beside(lane, here, sides[at].0);
                let xb = gutters.beside(lane, there, sides[at].1).0;
                // Sub-columns side by side, or one, share the gutter; further apart it runs above or below every part to the other's.
                let mut points = vec![a, Pt { x: xa, y: a.y }];
                if (xa - xb).abs() > 0.5 {
                    let level = |y: f32| (a.y - y).abs() + (b.y - y).abs();
                    let y = if level(gutters.over) <= level(gutters.under) { gutters.over } else { gutters.under };
                    points.extend([Pt { x: xa, y }, Pt { x: xb, y }]);
                }
                points.extend([Pt { x: xb, y: b.y }, b]);
                runs.push(Some((xa, out.x, room, a.y, points[2].y)));
                return Edge { from, to, label, step: map.links[wire].step, a, b, path: rounded(&points, metrics.gap * 0.75), lane: true, mid: a };
            }
            runs.push(None);
            let reach = if sides[at].0.vertical() { (b.y - a.y).abs() } else { (b.x - a.x).abs() };
            let pull = (reach * 0.5).max(PULL_LEAST);
            let c1 = Pt { x: a.x + out.x * pull, y: a.y + out.y * pull };
            let c2 = Pt { x: b.x + into.x * pull, y: b.y + into.y * pull };
            Edge { from, to, label, step: map.links[wire].step, a, b, path: vec![[a, c1, c2, b]], lane: false, mid: a }
        })
        .collect::<Vec<Edge>>();
    // Each pill takes the first place along its curve that is clear of every part and of the pills already set; a link inside a lane's
    // pill stands in its gutter.
    let mut placed: Vec<(Pt, f32)> = Vec::new();
    for (edge, run) in edges.iter_mut().zip(&runs) {
        edge.mid = match run {
            Some(run) => gutter_spot(edge, *run, nodes, &placed, metrics),
            None => label_spot(edge, nodes, &placed, metrics),
        };
        if edge.label.is_some() || edge.step.is_some() {
            placed.push((edge.mid, label_width(edge, metrics)));
        }
    }
    edges
}

/// An orthogonal run through `points` as cubic pieces, each corner turned on a quarter circle of at most `radius`.
fn rounded(points: &[Pt], radius: f32) -> Vec<[Pt; 4]> {
    // A control a quarter circle's cubic keeps from its ends, as a share of the radius.
    const KAPPA: f32 = 0.552;
    let mut points: Vec<Pt> = points.to_vec();
    points.dedup_by(|b, a| (a.x - b.x).abs() < 0.01 && (a.y - b.y).abs() < 0.01);
    let toward = |from: Pt, to: Pt| {
        let long = (to.x - from.x).hypot(to.y - from.y).max(0.001);
        (Pt { x: (to.x - from.x) / long, y: (to.y - from.y) / long }, long)
    };
    let straight = |from: Pt, to: Pt| [from, Pt { x: from.x + (to.x - from.x) / 3., y: from.y + (to.y - from.y) / 3. }, Pt { x: from.x + (to.x - from.x) * 2. / 3., y: from.y + (to.y - from.y) * 2. / 3. }, to];
    let mut path = Vec::new();
    let mut at = points[0];
    for corner in 1..points.len().saturating_sub(1) {
        let (before, here, after) = (points[corner - 1], points[corner], points[corner + 1]);
        let ((din, lin), (dout, lout)) = (toward(before, here), toward(here, after));
        let r = radius.min(lin / 2.).min(lout / 2.);
        let (enter, leave) = (Pt { x: here.x - din.x * r, y: here.y - din.y * r }, Pt { x: here.x + dout.x * r, y: here.y + dout.y * r });
        if (enter.x - at.x).abs() + (enter.y - at.y).abs() > 0.01 {
            path.push(straight(at, enter));
        }
        if r > 0.01 {
            path.push([enter, Pt { x: enter.x + din.x * r * KAPPA, y: enter.y + din.y * r * KAPPA }, Pt { x: leave.x - dout.x * r * KAPPA, y: leave.y - dout.y * r * KAPPA }, leave]);
        }
        at = leave;
    }
    if let Some(&end) = points.last() {
        if (end.x - at.x).abs() + (end.y - at.y).abs() > 0.01 || path.is_empty() {
            path.push(straight(at, end));
        }
    }
    path
}

/// Where a pill of a link inside a lane stands: on the run along its gutter, nearest the run's middle that is clear, and pushed
/// away from the parts as far as it is too wide for the gutter. `run` is the run's x, which way is away, the room to the part, and
/// where the run starts and ends down the block.
fn gutter_spot(edge: &Edge, run: (f32, f32, f32, f32, f32), nodes: &[Node], placed: &[(Pt, f32)], metrics: &Metrics) -> Pt {
    let (x, away, room, from, to) = run;
    let (wide, high) = (label_width(edge, metrics), metrics.node_plain / 2.);
    let x = x + away * (wide / 2. + metrics.gap / 4. - room).max(0.);
    let (top, bottom) = (from.min(to), from.max(to));
    let clear = |at: Pt| {
        nodes.iter().all(|node| at.x + wide / 2. <= node.rect.x || at.x - wide / 2. >= node.rect.right() || at.y + high / 2. <= node.rect.y || at.y - high / 2. >= node.rect.bottom())
            && placed.iter().all(|(other, other_wide)| (at.x - other.x).abs() >= (wide + other_wide) / 2. || (at.y - other.y).abs() >= high)
    };
    let middle = Pt { x, y: (top + bottom) / 2. };
    (0..=22).flat_map(|step| [0.5 + step as f32 * 0.02, 0.5 - step as f32 * 0.02]).map(|t| Pt { x, y: top + (bottom - top) * t }).find(|at| clear(*at)).unwrap_or(middle)
}

fn label_width(edge: &Edge, metrics: &Metrics) -> f32 {
    let chars = edge.label.as_deref().map_or(0, |label| label.chars().count()) as f32 + if edge.step.is_some() { 2. } else { 0. };
    chars * metrics.char + metrics.gap * 2.
}

/// Where along its curve a link's pill stands: the middle, or the nearest place that is clear of every part, so a curve that
/// passes over a column does not put its label on a part in it.
fn label_spot(edge: &Edge, nodes: &[Node], placed: &[(Pt, f32)], metrics: &Metrics) -> Pt {
    let (wide, high) = (label_width(edge, metrics), metrics.node_plain / 2.);
    let clear = |at: Pt| {
        nodes.iter().all(|node| at.x + wide / 2. <= node.rect.x || at.x - wide / 2. >= node.rect.right() || at.y + high / 2. <= node.rect.y || at.y - high / 2. >= node.rect.bottom())
            && placed.iter().all(|(other, other_wide)| (at.x - other.x).abs() >= (wide + other_wide) / 2. || (at.y - other.y).abs() >= high)
    };
    // From the middle outward, a step at a time, to either side.
    (0..=22).flat_map(|step| [0.5 + step as f32 * 0.02, 0.5 - step as f32 * 0.02]).map(|t| edge.at(t)).find(|at| clear(*at)).unwrap_or_else(|| edge.at(0.5))
}

/// What `hover` lights: the links that touch a part and the parts at their ends, on level one; on a deeper level everything the
/// part is chained to, up and down. A feature lights its parts and the links that leave them.
pub fn lit(arranged: &Arranged, map: &SystemMap, hover: &Hover, depth: usize) -> Option<Lit> {
    let mut lit = Lit::default();
    match hover {
        Hover::None => return None,
        Hover::Part(id) => {
            let at = arranged.index_of(id)?;
            lit.nodes.insert(at);
            if depth > 1 {
                // Follow links forward to what it reaches and backward to what reaches it, never turning round.
                for forward in [true, false] {
                    let mut seen: HashSet<usize> = HashSet::from([at]);
                    let mut grew = true;
                    while grew {
                        grew = false;
                        for (edge_at, edge) in arranged.edges.iter().enumerate() {
                            let (near, far) = if forward { (edge.from, edge.to) } else { (edge.to, edge.from) };
                            if seen.contains(&near) {
                                lit.edges.insert(edge_at);
                                if seen.insert(far) {
                                    grew = true;
                                }
                            }
                        }
                    }
                    lit.nodes.extend(seen);
                }
            } else {
                for (edge_at, edge) in arranged.edges.iter().enumerate() {
                    if edge.from == at || edge.to == at {
                        lit.edges.insert(edge_at);
                        lit.nodes.insert(edge.from);
                        lit.nodes.insert(edge.to);
                    }
                }
            }
        }
        Hover::Feature(slug) => {
            let feature = map.features.iter().find(|feature| feature.slug == *slug)?;
            let owned: HashSet<usize> = feature.parts.iter().filter_map(|id| arranged.index_of(id)).collect();
            lit.nodes.extend(owned.iter().copied());
            for (edge_at, edge) in arranged.edges.iter().enumerate() {
                if owned.contains(&edge.from) {
                    lit.edges.insert(edge_at);
                    lit.nodes.insert(edge.to);
                }
            }
        }
    }
    Some(lit)
}

/// Which links are drawn, by place in `arranged.edges`. A link between two lanes always is. One inside a lane is drawn only while a
/// part at one of its ends is hovered or selected, or the hovered feature holds one, since at rest a lane full of them is noise; a
/// numbered step of level three's path always is, the path being the point of that level.
pub fn shown(arranged: &Arranged, map: &SystemMap, hover: &Hover, selected: Option<&str>) -> Vec<bool> {
    let mut near: HashSet<usize> = selected.and_then(|id| arranged.index_of(id)).into_iter().collect();
    match hover {
        Hover::None => {}
        Hover::Part(id) => near.extend(arranged.index_of(id)),
        Hover::Feature(slug) => near.extend(map.features.iter().filter(|feature| feature.slug == *slug).flat_map(|feature| &feature.parts).filter_map(|id| arranged.index_of(id))),
    }
    arranged.edges.iter().map(|edge| !edge.lane || edge.step.is_some() || near.contains(&edge.from) || near.contains(&edge.to)).collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

/// The part an arrow key goes to from `from`: the nearest one lying that way, a part straight along ahead of one off to the side.
pub fn toward(nodes: &[Node], from: usize, dir: Dir) -> Option<usize> {
    let here = nodes.get(from)?.rect.centre();
    nodes
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != from)
        .filter_map(|(at, node)| {
            let there = node.rect.centre();
            let (along, across) = match dir {
                Dir::Right => (there.x - here.x, there.y - here.y),
                Dir::Left => (here.x - there.x, there.y - here.y),
                Dir::Down => (there.y - here.y, there.x - here.x),
                Dir::Up => (here.y - there.y, there.x - here.x),
            };
            // Parts in a column share an x; only one that is further along the way counts, and a nearer one beats a straighter one.
            (along > 1.).then(|| (at, along + across.abs() * 2.))
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(at, _)| at)
}

/// The part the keys start on: the top left one.
pub fn first(nodes: &[Node]) -> Option<usize> {
    nodes
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1.rect.x, a.1.rect.y).partial_cmp(&(b.1.rect.x, b.1.rect.y)).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(at, _)| at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::{PartOrigin, PathStop, SystemFeature, SystemLink};

    fn part(id: &str, kind: SystemPartKind) -> SystemPart {
        SystemPart { id: id.into(), name: id.into(), kind, summary: String::new(), within: None, manifest_id: None, paths: vec![format!("{id}/")], zooms: false, origin: PartOrigin::Detected { from: "cargo".into() } }
    }

    fn link(from: &str, to: &str, label: Option<&str>) -> SystemLink {
        SystemLink { from: from.into(), to: to.into(), label: label.map(str::to_string), step: None }
    }

    fn map(parts: Vec<SystemPart>, links: Vec<SystemLink>) -> SystemMap {
        SystemMap { project: "p".into(), part: None, feature: None, parts, links, features: Vec::new(), stops: Vec::new(), librarian_at: None }
    }

    fn level_one() -> SystemMap {
        map(
            vec![
                part("you", SystemPartKind::Person),
                part("app", SystemPartKind::Entry),
                part("cli", SystemPartKind::Entry),
                part("daemon", SystemPartKind::Service),
                part("db", SystemPartKind::Store),
                part("github", SystemPartKind::Outside),
            ],
            vec![link("you", "app", None), link("you", "cli", None), link("app", "daemon", Some("wire")), link("cli", "daemon", Some("HTTP")), link("daemon", "db", Some("reads, writes")), link("daemon", "github", Some("PRs"))],
        )
    }

    fn arranged(map: &SystemMap, depth: usize) -> Arranged {
        arrange(map, depth, "p", &|id| Some(id.to_string()), &Metrics::default(), 1100., 400.)
    }

    #[test]
    fn level_one_stands_people_then_entry_points_then_what_they_reach_then_stores_then_outside() {
        let map = level_one();
        let out = arranged(&map, 1);
        let x = |id: &str| out.nodes[out.index_of(id).unwrap()].rect.x;
        assert!(x("you") < x("app") && x("app") < x("daemon") && x("daemon") < x("db") && x("db") < x("github"), "left to right");
        assert_eq!(x("app"), x("cli"), "entry points share a column");
        let titles: Vec<&str> = out.lanes.iter().map(|lane| lane.title.as_str()).collect();
        assert_eq!(titles, ["People", "p", "Outside"]);
        assert_eq!(out.lanes[1].kind, LaneKind::Boundary);
        assert!(out.lanes[1].rect.x < x("app") && out.lanes[1].rect.right() > x("db"), "the boundary holds the parts inside it");
        assert!(out.lanes[2].rect.x >= out.lanes[1].rect.right(), "outside stands beyond the boundary");
    }

    #[test]
    fn a_column_nothing_stands_in_takes_no_room() {
        let map = map(vec![part("app", SystemPartKind::Entry), part("db", SystemPartKind::Store)], vec![link("app", "db", None)]);
        let out = arranged(&map, 1);
        let titles: Vec<&str> = out.lanes.iter().map(|lane| lane.title.as_str()).collect();
        assert_eq!(titles, ["p"], "no people and nothing outside: the boundary alone");
        let gap = out.nodes[1].rect.x - out.nodes[0].rect.right();
        assert!((gap - Metrics::default().col_gap).abs() < 0.01, "entry and store are neighbours, with no empty column between");
    }

    #[test]
    fn no_two_parts_overlap_and_all_stand_inside_the_block() {
        let out = arranged(&level_one(), 1);
        for (at, a) in out.nodes.iter().enumerate() {
            assert!(a.rect.x >= 0. && a.rect.right() <= out.width + 0.5 && a.rect.y >= 0. && a.rect.bottom() <= out.height, "{} is inside", a.id);
            for b in &out.nodes[at + 1..] {
                let apart = a.rect.right() <= b.rect.x || b.rect.right() <= a.rect.x || a.rect.bottom() <= b.rect.y || b.rect.bottom() <= a.rect.y;
                assert!(apart, "{} and {} overlap", a.id, b.id);
            }
        }
    }

    #[test]
    fn a_level_of_many_pieces_wraps_its_lane_and_keeps_the_linked_lanes_in_view() {
        let map: SystemMap = serde_json::from_str(include_str!("fixtures/fleet-level-two.json")).unwrap();
        let out = arrange_fitted(&map, 2, "Armada", &|id| Some(id.to_string()), &Metrics::default(), 1400., 820.);
        for (at, a) in out.nodes.iter().enumerate() {
            assert!(a.rect.x >= 0. && a.rect.right() <= out.width + 0.5 && a.rect.y >= 0. && a.rect.bottom() <= out.height, "{} is inside the room", a.id);
            for b in &out.nodes[at + 1..] {
                let apart = a.rect.right() <= b.rect.x || b.rect.right() <= a.rect.x || a.rect.bottom() <= b.rect.y || b.rect.bottom() <= a.rect.y;
                assert!(apart, "{} and {} overlap", a.id, b.id);
            }
        }
        let first = |id: &str| out.nodes[out.index_of(id).unwrap()].rect;
        assert!(first("store-armada-db").bottom() <= 820. && first("api/routes").bottom() <= 820. && first("outside-github").bottom() <= 820.);
    }

    #[test]
    fn beside_the_panel_the_pieces_take_fewer_columns_and_the_linked_lanes_still_stand_in_the_first_screen() {
        let map: SystemMap = serde_json::from_str(include_str!("fixtures/fleet-level-two.json")).unwrap();
        let out = arrange_fitted(&map, 2, "Armada", &|id| Some(id.to_string()), &Metrics::default(), 960., 600.);
        assert!(out.width <= 961., "it is no wider than its room: {}", out.width);
        for (at, a) in out.nodes.iter().enumerate() {
            for b in &out.nodes[at + 1..] {
                let apart = a.rect.right() <= b.rect.x || b.rect.right() <= a.rect.x || a.rect.bottom() <= b.rect.y || b.rect.bottom() <= a.rect.y;
                assert!(apart, "{} and {} overlap", a.id, b.id);
            }
        }
        for id in ["store-armada-db", "api/routes", "outside-github", "outside-cloudflare", "outside-claude-code", "outside-api-buildkite-com"] {
            let rect = out.nodes[out.index_of(id).unwrap()].rect;
            assert!(rect.bottom() <= 600., "{id} stands in the first screen: {}", rect.bottom());
        }
    }

    #[test]
    fn ports_on_a_side_are_sorted_by_where_the_far_end_stands() {
        let out = arranged(&level_one(), 1);
        let daemon = out.index_of("daemon").unwrap();
        let mut leaving: Vec<(f32, f32)> = out.edges.iter().filter(|edge| edge.from == daemon).map(|edge| (edge.a.y, out.nodes[edge.to].rect.centre().y)).collect();
        assert_eq!(leaving.len(), 2);
        leaving.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        assert!(leaving[0].1 <= leaving[1].1, "the port that stands higher goes to the one that stands higher");
        let arriving: Vec<f32> = out.edges.iter().filter(|edge| edge.to == daemon).map(|edge| edge.b.x).collect();
        assert!(arriving.iter().all(|x| (*x - out.nodes[daemon].rect.x).abs() < 0.01), "they arrive on its left side");
    }

    #[test]
    fn a_label_stands_at_the_middle_of_its_curve() {
        let out = arranged(&level_one(), 1);
        let edge = &out.edges[2];
        assert_eq!(edge.label.as_deref(), Some("wire"));
        let (a, b) = (edge.a, edge.b);
        assert!(edge.mid.x > a.x && edge.mid.x < b.x, "between its ends");
        assert!((edge.mid.y - (a.y + b.y) / 2.).abs() < 0.01, "level between them, as a curve with level ends is");
    }

    #[test]
    fn hovering_a_part_on_level_one_lights_the_links_that_touch_it_and_no_more() {
        let map = level_one();
        let out = arranged(&map, 1);
        let lit = lit(&out, &map, &Hover::Part("app".into()), 1).unwrap();
        let ids: HashSet<&str> = lit.nodes.iter().map(|at| out.nodes[*at].id.as_str()).collect();
        assert_eq!(ids, HashSet::from(["app", "you", "daemon"]));
        assert_eq!(lit.edges.len(), 2, "you to app, app to daemon");
        assert!(lit_none(&out, &map));
    }

    fn lit_none(out: &Arranged, map: &SystemMap) -> bool {
        lit(out, map, &Hover::None, 1).is_none()
    }

    #[test]
    fn hovering_a_part_deeper_down_lights_the_whole_chain_up_and_down() {
        let map = level_one();
        let out = arranged(&map, 2);
        let lit = lit(&out, &map, &Hover::Part("daemon".into()), 2).unwrap();
        let ids: HashSet<&str> = lit.nodes.iter().map(|at| out.nodes[*at].id.as_str()).collect();
        assert_eq!(ids, HashSet::from(["you", "app", "cli", "daemon", "db", "github"]), "back to where it starts and on to where it ends");
        let from_app = self::lit(&out, &map, &Hover::Part("app".into()), 2).unwrap();
        let ids: HashSet<&str> = from_app.nodes.iter().map(|at| out.nodes[*at].id.as_str()).collect();
        assert!(!ids.contains("cli"), "a chain does not turn round into a part beside it: {ids:?}");
    }

    #[test]
    fn a_feature_lights_its_parts_and_the_links_that_leave_them() {
        let mut map = level_one();
        map.features.push(SystemFeature { slug: "wire".into(), title: "Wire".into(), parts: vec!["app".into()] });
        let out = arranged(&map, 1);
        let lit = lit(&out, &map, &Hover::Feature("wire".into()), 1).unwrap();
        let ids: HashSet<&str> = lit.nodes.iter().map(|at| out.nodes[*at].id.as_str()).collect();
        assert_eq!(ids, HashSet::from(["app", "daemon"]));
        assert_eq!(lit.edges.len(), 1);
    }

    fn piece(id: &str, within: &str) -> SystemPart {
        SystemPart { within: Some(within.into()), ..part(id, SystemPartKind::Piece) }
    }

    /// A lane of `count` pieces of `a` and one piece of `b`.
    fn lanes(count: usize, links: Vec<SystemLink>) -> SystemMap {
        let mut parts: Vec<SystemPart> = (0..count).map(|at| piece(&format!("a{at}"), "a")).collect();
        parts.push(piece("b0", "b"));
        map(parts, links)
    }

    /// No point of `edge`'s curve between its ends stands inside a part.
    fn crosses_no_part(out: &Arranged, edge: &Edge) -> bool {
        (1..400).map(|at| edge.at(at as f32 / 400.)).all(|pt| out.nodes.iter().all(|node| !(pt.x > node.rect.x + 0.5 && pt.x < node.rect.right() - 0.5 && pt.y > node.rect.y + 0.5 && pt.y < node.rect.bottom() - 0.5)))
    }

    fn edge_of<'a>(out: &'a Arranged, from: &str, to: &str) -> (usize, &'a Edge) {
        let (from, to) = (out.index_of(from).unwrap(), out.index_of(to).unwrap());
        out.edges.iter().enumerate().find(|(_, edge)| edge.from == from && edge.to == to).unwrap()
    }

    #[test]
    fn a_link_inside_a_lane_is_drawn_only_while_a_part_at_one_of_its_ends_is_hovered_or_selected() {
        let map = lanes(3, vec![link("a0", "a2", Some("reads")), link("a0", "b0", None)]);
        let out = arranged(&map, 2);
        let (inside, edge) = edge_of(&out, "a0", "a2");
        assert!(edge.lane);
        let (between, edge) = edge_of(&out, "a0", "b0");
        assert!(!edge.lane);
        let shown_for = |hover: Hover, selected: Option<&str>| shown(&out, &map, &hover, selected);
        let rest = shown_for(Hover::None, None);
        assert!(!rest[inside] && rest[between], "at rest the link between lanes is drawn and the one inside a lane is not");
        assert!(!shown_for(Hover::Part("a1".into()), None)[inside], "a part it would run past does not draw it");
        assert!(shown_for(Hover::Part("a2".into()), None)[inside], "the part at its far end does");
        assert!(shown_for(Hover::None, Some("a0"))[inside], "and so does the selected part at its near end");
    }

    #[test]
    fn a_link_inside_a_lane_runs_out_to_the_gutter_and_back_and_crosses_no_part_and_its_label_stands_clear() {
        let map = lanes(4, vec![link("a0", "a3", Some("reads")), link("a3", "a1", None), link("a0", "b0", Some("calls"))]);
        let out = arranged(&map, 2);
        for edge in out.edges.iter().filter(|edge| edge.lane) {
            assert!(crosses_no_part(&out, edge), "{} to {} crosses no part", out.nodes[edge.from].id, out.nodes[edge.to].id);
            assert!(edge.path.len() > 1, "it turns out of the part and back in");
        }
        let (_, edge) = edge_of(&out, "a0", "a3");
        let wide = label_width(edge, &Metrics::default());
        let column = out.nodes[edge.from].rect;
        assert!(edge.mid.x - wide / 2. >= column.right() || edge.mid.x + wide / 2. <= column.x, "its label stands in the gutter, off the column: {:?}", edge.mid);
        let (_, between) = edge_of(&out, "a0", "b0");
        assert_eq!(between.path.len(), 1, "a link between lanes is the one curve it was");
    }

    #[test]
    fn pieces_in_neighbouring_sub_columns_of_a_wrapped_lane_are_still_one_lane() {
        let map = lanes(12, vec![link("a0", "a11", None), link("a0", "a6", None), link("a11", "a0", None), link("a0", "b0", None)]);
        let out = arrange(&map, 2, "p", &|id| Some(id.to_string()), &Metrics::default(), 1600., 400.);
        let x = |id: &str| out.nodes[out.index_of(id).unwrap()].rect.x;
        assert!(x("a0") < x("a6") && x("a6") < x("a11"), "the lane wrapped into three sub-columns");
        let rest = shown(&out, &map, &Hover::None, None);
        for (at, edge) in out.edges.iter().enumerate() {
            let one_lane = out.nodes[edge.to].id != "b0";
            assert_eq!(edge.lane, one_lane, "{} is inside the lane", out.nodes[edge.to].id);
            assert_eq!(rest[at], !one_lane);
            assert!(crosses_no_part(&out, edge) || !one_lane, "{} to {} crosses no part", out.nodes[edge.from].id, out.nodes[edge.to].id);
        }
    }

    #[test]
    fn level_three_draws_its_numbered_steps_at_rest_even_inside_a_lane_and_by_its_gutter() {
        let mut map = lanes(3, vec![SystemLink { step: Some(1), ..link("a0", "a2", Some("asks")) }, SystemLink { step: Some(2), ..link("a2", "b0", None) }]);
        map.feature = Some("f".into());
        let out = arranged(&map, 3);
        let (inside, edge) = edge_of(&out, "a0", "a2");
        assert!(edge.lane && crosses_no_part(&out, edge));
        let rest = shown(&out, &map, &Hover::None, None);
        assert!(rest.iter().all(|one| *one), "every step of the path is drawn at rest");
        assert!(rest[inside]);
    }

    #[test]
    fn level_one_keeps_its_links_inside_the_boundary_as_they_were() {
        let out = arranged(&level_one(), 1);
        assert!(out.edges.iter().all(|edge| !edge.lane && edge.path.len() == 1));
    }

    #[test]
    fn level_two_stands_pieces_in_a_lane_for_the_part_each_belongs_to_with_stores_last() {
        let mut bridge = part("bridge/board", SystemPartKind::Piece);
        bridge.within = Some("bridge".into());
        let mut fleet = part("fleet/board", SystemPartKind::Piece);
        fleet.within = Some("fleet".into());
        let store = part("sqlite", SystemPartKind::Store);
        let mut map = map(vec![store, fleet, bridge], vec![link("bridge/board", "fleet/board", Some("get_board")), link("fleet/board", "sqlite", None)]);
        map.part = Some("bridge".into());
        let out = arrange(&map, 2, "p", &|id| Some(id.to_uppercase()), &Metrics::default(), 1100., 300.);
        let titles: Vec<&str> = out.lanes.iter().map(|lane| lane.title.as_str()).collect();
        assert_eq!(titles, ["BRIDGE", "FLEET", "Stores"], "the lane that reaches the others first, the stores last");
    }

    #[test]
    fn level_three_puts_a_features_files_under_the_part_that_holds_them_and_takes_the_room() {
        let mut piece = part("bridge/board", SystemPartKind::Piece);
        piece.within = Some("bridge".into());
        let mut map = map(vec![piece], Vec::new());
        map.feature = Some("dashboard".into());
        map.stops.push(PathStop { step: 1, part: "bridge/board".into(), files: vec!["view.rs".into(), "layout.rs".into()] });
        let out = arranged(&map, 3);
        assert_eq!(out.nodes[0].files, ["view.rs", "layout.rs"]);
        assert!(out.nodes[0].rect.h > Metrics::default().node_plain + 2. * Metrics::default().file);
        assert_eq!(out.nodes[0].meta, None, "the files stand where the line under the name was");
    }

    #[test]
    fn an_arrow_key_goes_to_the_nearest_part_that_way_and_none_where_there_is_nothing() {
        let map = level_one();
        let out = arranged(&map, 1);
        let id = |at: usize| out.nodes[at].id.as_str();
        let app = out.index_of("app").unwrap();
        assert_eq!(toward(&out.nodes, app, Dir::Right).map(id), Some("daemon"));
        assert_eq!(toward(&out.nodes, app, Dir::Left).map(id), Some("you"));
        assert_eq!(toward(&out.nodes, app, Dir::Down).map(id), Some("cli"));
        assert_eq!(toward(&out.nodes, app, Dir::Up), None, "nothing above the top one of its column");
        let you = out.index_of("you").unwrap();
        assert_eq!(toward(&out.nodes, you, Dir::Left), None);
        assert_eq!(first(&out.nodes).map(id), Some("you"));
    }

    #[test]
    fn a_link_to_a_part_that_is_not_on_the_level_is_left_out_and_an_empty_map_still_has_a_size() {
        let map = map(vec![part("a", SystemPartKind::Entry)], vec![link("a", "gone", None)]);
        let out = arranged(&map, 1);
        assert!(out.edges.is_empty());
        let none = arranged(&super::tests::map(Vec::new(), Vec::new()), 1);
        assert_eq!((none.width, none.height), (1100., 400.));
    }

    #[test]
    fn a_room_too_narrow_for_the_parts_shrinks_the_graph_to_fit_down_to_the_floor_and_scrolls_below_it() {
        let map = level_one();
        let fitted = |width: f32| arrange_fitted(&map, 1, "p", &|id| Some(id.to_string()), &Metrics::default(), width, 0.);
        let natural = fitted(5000.);
        assert_eq!(natural.scale, 1.);
        let need = arrange(&map, 1, "p", &|id| Some(id.to_string()), &Metrics::default(), 0., 0.).width;
        let tight = fitted(need * 0.85);
        assert!(tight.scale < 1. && tight.scale >= FLOOR);
        assert!(tight.width <= need * 0.85 + 1., "it fits: {} in {}", tight.width, need * 0.85);
        let narrow = fitted(need * 0.5);
        assert_eq!(narrow.scale, FLOOR);
        assert!(narrow.width > need * 0.5, "below the floor it scrolls");
    }

    #[test]
    fn a_part_is_as_wide_as_its_name_and_path_need() {
        let mut long = part("app", SystemPartKind::Entry);
        long.paths = vec!["crates/bridge-native/".into()];
        let map = map(vec![long], Vec::new());
        let out = arrange(&map, 1, "p", &|id| Some(id.to_string()), &Metrics::default(), 0., 0.);
        let metrics = Metrics::default();
        assert!(out.nodes[0].rect.w >= "crates/bridge-native".len() as f32 * metrics.char + metrics.chrome);
    }
}
