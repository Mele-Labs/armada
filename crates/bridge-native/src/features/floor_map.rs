//! The floor plan as a map, worked out once for each read of a page: every module a plot sized by its bytes, every file a pill
//! inside it sized by its own, nested by folder, and the contract doors as lines between plots. A squarified treemap, as the
//! approved mock (`Sized pills with contracts`) lays it out, in pixels at the size the card gives it. The card lays it out again
//! only when its width changes, never on a frame alone.

use std::collections::BTreeMap;

use ipc::{FloorFile, FloorPlan, FloorRoom};

/// The mock's size, whose shape the card keeps.
pub const WIDTH: f32 = 640.;
pub const HEIGHT: f32 = 330.;
/// The gap round a plot, inside its share of the map.
const PLOT_GAP: f32 = 3.;
/// The smallest area a file or folder gets a pill of its own at; smaller ones in one folder share a pill. Ones the feature owns or a
/// Job is changing always stand alone, at no less than this.
const LEAST_AREA: f32 = 55.;
/// A pill thinner than this is not drawn.
const LEAST_SIDE: f32 = 1.5;
/// What a pill gives up on each side, so neighbours read apart.
const PILL_INSET: f32 = 0.7;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    fn area(&self) -> f32 {
        self.w * self.h
    }

    fn centre(&self) -> (f32, f32) {
        (self.x + self.w / 2., self.y + self.h / 2.)
    }

    fn inset(&self, by: f32) -> Rect {
        Rect { x: self.x + by, y: self.y + by, w: self.w - 2. * by, h: self.h - 2. * by }
    }
}

/// How a pill is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Plain,
    /// The feature owns it.
    Lit,
    /// A live Job has written it.
    Changing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plot {
    /// The module's name as drawn: its last path segment.
    pub name: String,
    pub lit: bool,
    pub rect: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pill {
    pub rect: Rect,
    pub ink: Ink,
}

/// A contract door: from the centre of one plot to the centre of the one it leads to, named for that one.
#[derive(Clone, Debug, PartialEq)]
pub struct Door {
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub label: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FloorMap {
    pub plots: Vec<Plot>,
    pub pills: Vec<Pill>,
    pub doors: Vec<Door>,
}

/// What VoiceOver reads for the whole map.
pub fn said(plan: &FloorPlan) -> String {
    let lit = plan.rooms.iter().filter(|room| room.lit).count();
    let modules = if plan.rooms.len() == 1 { "module" } else { "modules" };
    let holds = if lit == 1 { "holds" } else { "hold" };
    format!("Floor plan: {lit} of {} {modules} {holds} this feature's files", plan.rooms.len())
}

/// Lay `items`, each a key and a weight, largest first, into `within` so each gets an area in proportion to its weight, as
/// squarely as rows allow (Bruls, Huizing and van Wijk). Items of no weight get no place.
pub fn squarify<K: Clone>(items: &[(K, f64)], within: Rect) -> Vec<(K, Rect)> {
    let mut out = Vec::new();
    let total: f64 = items.iter().map(|(_, weight)| weight.max(0.)).sum();
    if total <= 0. || within.w <= 0. || within.h <= 0. {
        return out;
    }
    let scale = within.area() as f64 / total;
    let mut rest: Vec<(K, f64)> = items.iter().filter(|(_, weight)| *weight > 0.).map(|(key, weight)| (key.clone(), weight * scale)).collect();
    let (mut x, mut y, mut w, mut h) = (within.x as f64, within.y as f64, within.w as f64, within.h as f64);
    while !rest.is_empty() {
        let short = w.min(h);
        let worst = |row: &[(K, f64)]| {
            let sum: f64 = row.iter().map(|(_, area)| area).sum();
            row.iter().map(|(_, area)| (short * short * area / (sum * sum)).max(sum * sum / (short * short * area))).fold(0., f64::max)
        };
        let mut taken = 1;
        let mut best = worst(&rest[..1]);
        while taken < rest.len() {
            let next = worst(&rest[..taken + 1]);
            if next > best {
                break;
            }
            best = next;
            taken += 1;
        }
        let row: Vec<(K, f64)> = rest.drain(..taken).collect();
        let sum: f64 = row.iter().map(|(_, area)| area).sum();
        if w >= h {
            let column = sum / h;
            let mut at = y;
            for (key, area) in row {
                let tall = area / column;
                out.push((key, Rect { x: x as f32, y: at as f32, w: column as f32, h: tall as f32 }));
                at += tall;
            }
            x += column;
            w -= column;
        } else {
            let line = sum / w;
            let mut at = x;
            for (key, area) in row {
                let wide = area / line;
                out.push((key, Rect { x: at as f32, y: y as f32, w: wide as f32, h: line as f32 }));
                at += wide;
            }
            y += line;
            h -= line;
        }
    }
    out
}

/// A folder of a room: its files and the folders under it.
#[derive(Default)]
struct Folder<'a> {
    folders: BTreeMap<String, Folder<'a>>,
    files: Vec<&'a FloorFile>,
}

impl Folder<'_> {
    fn bytes(&self) -> u64 {
        self.files.iter().map(|file| file.bytes).sum::<u64>() + self.folders.values().map(Folder::bytes).sum::<u64>()
    }

    fn all(&self) -> Vec<&FloorFile> {
        self.files.iter().copied().chain(self.folders.values().flat_map(Folder::all)).collect()
    }
}

fn ink_of<'a>(files: impl IntoIterator<Item = &'a FloorFile>) -> Ink {
    let mut ink = Ink::Plain;
    for file in files {
        if file.changing {
            return Ink::Changing;
        }
        if file.lit {
            ink = Ink::Lit;
        }
    }
    ink
}

fn marked(file: &FloorFile) -> bool {
    file.lit || file.changing
}

/// A file's path inside its room: under the room's folder, or the whole path for a room at the repository's root.
fn inside<'a>(room: &FloorRoom, path: &'a str) -> &'a str {
    if room.path == "." || room.path.is_empty() {
        return path;
    }
    path.strip_prefix(room.path.trim_end_matches('/')).and_then(|rest| rest.strip_prefix('/')).unwrap_or(path)
}

/// The room's files as a tree of folders. A taken-together entry (`grouped`, its path a folder) sits in the folder it names.
fn tree(room: &FloorRoom) -> Folder<'_> {
    let mut root = Folder::default();
    for file in &room.files {
        let path = inside(room, &file.path);
        let mut parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
        if file.grouped == 0 {
            parts.pop();
        }
        let mut folder = &mut root;
        for part in parts {
            folder = folder.folders.entry(part.to_string()).or_default();
        }
        folder.files.push(file);
    }
    root
}

enum Item<'a, 'b> {
    File(&'a FloorFile),
    Folder(&'b Folder<'a>),
    Rest(Vec<&'a FloorFile>),
}

/// The pills of one folder inside `within`. `per_unit` is bytes for one unit of area in this room.
fn lay(folder: &Folder<'_>, within: Rect, per_unit: f64, pills: &mut Vec<Pill>) {
    let mut big: Vec<(Item, f64)> = Vec::new();
    let mut small: Vec<&FloorFile> = Vec::new();
    let mut weighed: Vec<(Item, u64, bool)> = folder.files.iter().map(|file| (Item::File(*file), file.bytes, marked(file))).collect();
    weighed.extend(folder.folders.values().map(|sub| (Item::Folder(sub), sub.bytes(), sub.all().into_iter().any(marked))));
    let least = LEAST_AREA as f64 * per_unit;
    for (item, bytes, keep) in weighed {
        if keep {
            big.push((item, (bytes as f64).max(least)));
        } else if bytes as f64 >= least {
            big.push((item, bytes as f64));
        } else {
            match item {
                Item::File(file) => small.push(file),
                Item::Folder(sub) => small.extend(sub.all()),
                Item::Rest(_) => {}
            }
        }
    }
    if !small.is_empty() {
        let bytes = small.iter().map(|file| file.bytes).sum::<u64>() as f64;
        big.push((Item::Rest(small), bytes));
    }
    big.sort_by(|a, b| b.1.total_cmp(&a.1));
    let keyed: Vec<(usize, f64)> = big.iter().enumerate().map(|(at, (_, bytes))| (at, *bytes)).collect();
    for (at, rect) in squarify(&keyed, within) {
        match &big[at].0 {
            Item::File(file) => pill(rect, ink_of([*file]), pills),
            Item::Rest(files) => pill(rect, ink_of(files.iter().copied()), pills),
            Item::Folder(sub) => lay(sub, rect, per_unit, pills),
        }
    }
}

/// A sliver beside a much larger file is left out unless the feature owns it or a Job is changing it; those draw at least a line wide.
fn pill(rect: Rect, ink: Ink, pills: &mut Vec<Pill>) {
    if ink == Ink::Plain && (rect.w < LEAST_SIDE || rect.h < LEAST_SIDE) {
        return;
    }
    let rect = Rect { x: rect.x + PILL_INSET, y: rect.y + PILL_INSET, w: (rect.w - 2. * PILL_INSET).max(1.), h: (rect.h - 2. * PILL_INSET).max(1.) };
    pills.push(Pill { rect, ink });
}

/// A module's name as a plot draws it: the last segment of its path.
fn short(module: &str) -> String {
    module.trim_end_matches('/').rsplit('/').next().unwrap_or(module).to_string()
}

/// The whole map of `plan`, `width` by `height`, each plot's name in a strip `strip` tall at its top. A room with no files listed is
/// still a plot, sized as the smallest that has some.
pub fn map(plan: &FloorPlan, width: f32, height: f32, strip: f32) -> FloorMap {
    let weights: Vec<u64> = plan.rooms.iter().map(|room| room.files.iter().map(|file| file.bytes).sum()).collect();
    let least = weights.iter().copied().filter(|bytes| *bytes > 0).min().unwrap_or(1);
    let mut order: Vec<(usize, f64)> = weights.iter().enumerate().map(|(at, bytes)| (at, (*bytes).max(least) as f64)).collect();
    order.sort_by(|a, b| b.1.total_cmp(&a.1));
    let boxes = squarify(&order, Rect { x: 0., y: 0., w: width, h: height });
    let mut map = FloorMap::default();
    let mut centres: BTreeMap<&str, (f32, f32)> = BTreeMap::new();
    for (at, cell) in boxes {
        let room = &plan.rooms[at];
        centres.insert(room.module.as_str(), cell.centre());
        let plot = cell.inset(PLOT_GAP);
        map.plots.push(Plot { name: short(&room.module), lit: room.lit, rect: plot });
        let inner = Rect { x: plot.x + 3., y: plot.y + strip, w: plot.w - 6., h: plot.h - strip - 3. };
        if weights[at] == 0 || inner.w <= 0. || inner.h <= 0. {
            continue;
        }
        let per_unit = weights[at] as f64 / inner.area().max(1.) as f64;
        lay(&tree(room), inner, per_unit, &mut map.pills);
    }
    for door in plan.doors.iter().filter(|door| door.contract) {
        if let (Some(from), Some(to)) = (centres.get(door.from.as_str()), centres.get(door.to.as_str())) {
            map.doors.push(Door { from: *from, to: *to, label: short(&door.to) });
        }
    }
    map
}

/// A plan and its map, laid out for the last width it was drawn at.
#[derive(Debug, PartialEq)]
pub struct Laid {
    pub plan: FloorPlan,
    pub said: String,
    at: std::cell::RefCell<Option<(f32, std::rc::Rc<FloorMap>)>>,
}

impl Laid {
    pub fn new(plan: FloorPlan) -> Laid {
        Laid { said: said(&plan), plan, at: std::cell::RefCell::new(None) }
    }

    /// The map at `width`, laid out again only where the width moved.
    pub fn at(&self, width: f32, height: f32, strip: f32) -> std::rc::Rc<FloorMap> {
        let mut held = self.at.borrow_mut();
        match held.as_ref() {
            Some((was, map)) if (*was - width).abs() < 0.5 => map.clone(),
            _ => {
                let map = std::rc::Rc::new(map(&self.plan, width, height, strip));
                *held = Some((width, map.clone()));
                map
            }
        }
    }
}
