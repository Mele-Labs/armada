//! `--shot-check <scene.html>`: the screen a Job built, shot on the walk's mock data, set against the
//! picture of each state of the scene the owner approved, so a build that drifted from the scene is
//! flagged to the Job before it is offered.
//!
//! **Pairing.** A scene names its walk, `<scene walk="…">`, and each `<state id>` pairs with the step of
//! that walk whose [`crate::walk::Step::id`] is the same. A state with no such step is *missing*, which
//! fails the check: a build the walk cannot show in a state has not shown the state. `compare` scores each
//! pair; this file draws, pairs, writes `report.json` and the pictures, and says whether the check passed.
//! `docs/concepts/scenes.md`, *Shot check before an offer*.

pub mod compare;
pub mod look;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui::{px, size, AnyWindowHandle, AppContext, Entity, HeadlessAppContext};
use image::RgbaImage;
use serde::Serialize;

use crate::prototype::markup::{self, Scene};
use crate::prototype::{Home, PrototypeView, Source};
use crate::walk::Walk;

/// The walk window's size in points, as `--walk --check` opens it unless told otherwise.
const WALK_WIDTH: f32 = 1100.;
const WALK_HEIGHT: f32 = 820.;
/// The size `--against` draws both scenes at: the pop-out window's own.
const SCENE_WIDTH: f32 = 960.;
const SCENE_HEIGHT: f32 = 720.;

/// What the command line gave.
pub struct Args {
    pub scene: PathBuf,
    pub fixtures_dir: Option<PathBuf>,
    pub out: PathBuf,
    /// Another scene to draw in the walk's place: how the tolerance is measured, and how a revised scene is set against the approved one.
    pub against: Option<PathBuf>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

/// What became of one state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Matched,
    Drifted,
    /// No step of the walk is for this state.
    Missing,
}

/// One state of the scene and what it was set against.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Pair {
    pub state: String,
    pub status: Status,
    /// The walk's step, counted from 1 as the walk's own messages count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mean: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side_by_side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difference: Option<String>,
}

/// `report.json`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Report {
    pub scene: String,
    pub walk: String,
    pub ok: bool,
    pub mean_limit: f32,
    pub share_limit: f32,
    pub pairs: Vec<Pair>,
}

impl Report {
    pub fn new(scene: &str, walk: &str, pairs: Vec<Pair>) -> Report {
        let ok = pairs.iter().all(|pair| pair.status == Status::Matched);
        Report { scene: scene.to_string(), walk: walk.to_string(), ok, mean_limit: compare::MEAN_LIMIT, share_limit: compare::SHARE_LIMIT, pairs }
    }
}

/// Each state of the scene with the index of the walk's step that carries its id.
pub fn pair(states: &[String], walk: &Walk) -> Vec<(String, Option<usize>)> {
    states.iter().map(|state| (state.clone(), walk.steps.iter().position(|step| step.id == Some(state.as_str())))).collect()
}

/// The scene file and the fixtures beside it, as Fleet would send them.
fn read_source(path: &Path, fixtures_dir: Option<&Path>) -> Result<(Source, Scene), String> {
    let markup = std::fs::read_to_string(path).map_err(|why| format!("{}: {why}", path.display()))?;
    let scene = markup::parse(&markup)?;
    let dir = fixtures_dir.or(path.parent()).unwrap_or(Path::new("."));
    let mut fixtures = Vec::new();
    for src in scene.fixtures.iter().filter_map(|decl| decl.src.as_deref()) {
        let file = dir.join(src);
        fixtures.push((src.to_string(), std::fs::read_to_string(&file).map_err(|why| format!("{}: {why}", file.display()))?));
    }
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    Ok((Source { scene: name, markup, fixtures, state: None, trouble: None }, scene))
}

fn headless() -> HeadlessAppContext {
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    cx
}

/// A picture of each of `states` the scene declares, drawn alone at `size` points.
fn scene_pictures(source: Source, states: &[String], (width, height): (f32, f32)) -> Result<BTreeMap<String, RgbaImage>, String> {
    let mut cx = headless();
    let held: Rc<RefCell<Option<Entity<PrototypeView>>>> = Rc::default();
    let keep = held.clone();
    let handle: AnyWindowHandle = cx
        .open_window(size(px(width), px(height)), move |_, cx| {
            let view = cx.new(|cx| PrototypeView::new(source, Home::Shot, cx));
            *keep.borrow_mut() = Some(view.clone());
            view
        })
        .map_err(|why| why.to_string())?
        .into();
    let view = held.borrow().clone().ok_or("the scene's window did not open")?;
    cx.update_window(handle, |_, window, _| window.activate_window()).map_err(|why| why.to_string())?;
    let declared: Vec<String> = cx.update_window(handle, |_, _, cx| view.read(cx).declared()).map_err(|why| why.to_string())?;
    let mut pictures = BTreeMap::new();
    for state in states.iter().filter(|state| declared.contains(state)) {
        cx.update_window(handle, |_, _, cx| view.update(cx, |view, cx| view.go(state, cx))).map_err(|why| why.to_string())?;
        crate::prototype::shots::draw(&mut cx, handle, 3)?;
        pictures.insert(state.clone(), cx.capture_screenshot(handle).map_err(|why| why.to_string())?);
    }
    Ok(pictures)
}

/// The built screen at each step in `wanted`, cut to where the walk draws the screen, with that area's size in points.
fn walk_pictures(walk: &'static Walk, wanted: &[usize], width: f32, height: f32) -> Result<(BTreeMap<usize, RgbaImage>, (f32, f32)), String> {
    let mut found = BTreeMap::new();
    let mut area = None;
    let mut cut = |at: usize, picture: &RgbaImage, stage: Option<gpui::Bounds<gpui::Pixels>>| {
        if !wanted.contains(&at) {
            return;
        }
        let Some(stage) = stage else {
            return;
        };
        let scale = picture.width() as f32 / width;
        let (x, y) = ((f32::from(stage.origin.x) * scale).round() as u32, (f32::from(stage.origin.y) * scale).round() as u32);
        let (w, h) = ((f32::from(stage.size.width) * scale).round() as u32, (f32::from(stage.size.height) * scale).round() as u32);
        let (w, h) = (w.min(picture.width().saturating_sub(x)), h.min(picture.height().saturating_sub(y)));
        area.get_or_insert((f32::from(stage.size.width), f32::from(stage.size.height)));
        found.insert(at, image::imageops::crop_imm(picture, x, y, w, h).to_image());
    };
    crate::walk::check::play(walk, Vec::new(), None, None, None, Some(width), Some(height), Some(&mut cut))?;
    let area = area.ok_or_else(|| format!("the {} walk drew no screen at any step the scene's states name", walk.name))?;
    Ok((found, area))
}

/// `<stem>-side-by-side.png` and `<stem>-difference.png` of two pictures, written to `out`; their names.
fn write_pair(out: &Path, stem: &str, left: &RgbaImage, right: &RgbaImage) -> Result<(String, String), String> {
    let (both, gap) = (format!("{stem}-side-by-side.png"), format!("{stem}-difference.png"));
    for (name, picture) in [(&both, compare::side_by_side(left, right)), (&gap, compare::difference(left, right))] {
        let path = out.join(name);
        picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok((both, gap))
}

fn slug(state: &str) -> String {
    state.chars().map(|one| if one.is_ascii_alphanumeric() || one == '-' || one == '_' { one } else { '_' }).collect()
}

/// Draw, pair, compare and write. `Ok` is a report, which may say the check failed.
pub fn run(args: &Args) -> Result<Report, String> {
    std::fs::create_dir_all(&args.out).map_err(|why| format!("{}: {why}", args.out.display()))?;
    let (source, scene) = read_source(&args.scene, args.fixtures_dir.as_deref())?;
    let scene_name = source.scene.clone();

    // What each state is set against: the step of the walk, or the same state of another scene.
    let (walk_name, against, area): (String, BTreeMap<String, (Option<usize>, RgbaImage)>, (f32, f32)) = match &args.against {
        Some(other) => {
            let (other_source, _) = read_source(other, args.fixtures_dir.as_deref())?;
            let area = (args.width.unwrap_or(SCENE_WIDTH), args.height.unwrap_or(SCENE_HEIGHT));
            let drawn = scene_pictures(other_source.clone(), &scene.states, area)?;
            (other_source.scene, drawn.into_iter().map(|(state, picture)| (state, (None, picture))).collect(), area)
        }
        None => {
            let name = scene.walk.as_deref().ok_or("the scene names no walk: add walk=\"<walk name>\" to its <scene>")?;
            let walk = crate::walk::named(name).ok_or_else(|| format!("the scene names the walk {name}, and this build plays no such walk"))?;
            let paired = pair(&scene.states, walk);
            let wanted: Vec<usize> = paired.iter().filter_map(|(_, at)| *at).collect();
            if wanted.is_empty() {
                (name.to_string(), BTreeMap::new(), (0., 0.))
            } else {
                let (mut pictures, area) = walk_pictures(walk, &wanted, args.width.unwrap_or(WALK_WIDTH), args.height.unwrap_or(WALK_HEIGHT))?;
                let against = paired.into_iter().filter_map(|(state, at)| at.and_then(|at| pictures.remove(&at).map(|picture| (state, (Some(at + 1), picture))))).collect();
                (name.to_string(), against, area)
            }
        }
    };

    let mut pictures = if against.is_empty() { BTreeMap::new() } else { scene_pictures(source, &scene.states, area)? };
    let mut pairs = Vec::new();
    for state in &scene.states {
        let (Some((step, built)), Some(shown)) = (against.get(state), pictures.remove(state)) else {
            pairs.push(Pair { state: state.clone(), status: Status::Missing, step: None, mean: None, share: None, side_by_side: None, difference: None });
            continue;
        };
        let score = compare::score(&shown, built);
        let (both, gap) = write_pair(&args.out, &slug(state), &shown, built)?;
        let status = if score.drifted() { Status::Drifted } else { Status::Matched };
        pairs.push(Pair { state: state.clone(), status, step: *step, mean: Some(score.mean), share: Some(score.share), side_by_side: Some(both), difference: Some(gap) });
    }
    let report = Report::new(&scene_name, &walk_name, pairs);
    let path = args.out.join("report.json");
    let text = serde_json::to_string_pretty(&report).map_err(|why| why.to_string())?;
    std::fs::write(&path, text).map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::walk::{Screen, Step, Target, Verb};

    const fn step(id: Option<&'static str>) -> Step {
        Step { id, say: "A step", verb: Verb::Look, target: Target { role: "button", name: "Send" } }
    }

    static BUILT: Walk = Walk {
        name: "built-screen",
        title: "A built screen",
        screen: Screen::Jobs,
        scenario: "jobs-open",
        steps: &[step(None), step(Some("list")), step(None), step(Some("detail"))],
    };

    fn states(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    #[test]
    fn a_state_pairs_with_the_step_of_its_id() {
        assert_eq!(pair(&states(&["list", "detail"]), &BUILT), vec![("list".to_string(), Some(1)), ("detail".to_string(), Some(3))]);
    }

    #[test]
    fn a_state_no_step_is_for_is_missing() {
        assert_eq!(pair(&states(&["list", "empty"]), &BUILT), vec![("list".to_string(), Some(1)), ("empty".to_string(), None)]);
    }

    fn row(state: &str, status: Status) -> Pair {
        let seen = status != Status::Missing;
        Pair {
            state: state.into(),
            status,
            step: seen.then_some(2),
            mean: seen.then_some(0.0004),
            share: seen.then_some(0.),
            side_by_side: seen.then(|| format!("{state}-side-by-side.png")),
            difference: seen.then(|| format!("{state}-difference.png")),
        }
    }

    #[test]
    fn the_report_names_each_state_and_fails_for_one_that_did_not_match() {
        let report = Report::new("ready.html", "built-screen", vec![row("list", Status::Matched), row("detail", Status::Drifted), row("empty", Status::Missing)]);
        assert!(!report.ok);
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["scene"], "ready.html");
        assert_eq!(json["walk"], "built-screen");
        assert_eq!(json["ok"], false);
        let statuses: Vec<_> = json["pairs"].as_array().unwrap().iter().map(|one| (one["state"].as_str().unwrap(), one["status"].as_str().unwrap())).collect();
        assert_eq!(statuses, vec![("list", "matched"), ("detail", "drifted"), ("empty", "missing")]);
        assert_eq!(json["pairs"][1]["step"], 2);
        assert!(json["pairs"][1]["mean"].is_number());
        assert!(json["pairs"][2].get("mean").is_none(), "a missing state has no score");
        assert!(json["pairs"][0]["side_by_side"].is_string());
    }

    #[test]
    fn a_report_of_matches_alone_passes() {
        assert!(Report::new("ready.html", "built-screen", vec![row("list", Status::Matched)]).ok);
    }
}
