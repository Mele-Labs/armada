//! What Bridge asks of a page when a walk's step aims at something in it, ported from Armada
//! Classic's `main/capture/finder.ts`. **The page is told nothing**: the finder is a script evaluated in a
//! content world of its own, which shares the page's DOM and nothing else, and every answer is a small
//! plain value that is bounded here before it is used.

use ipc::WalkTarget;
use serde::Deserialize;

use super::frame::Frame;

/// The finder script. Installing it twice does nothing, so every ask carries it.
const SOURCE: &str = include_str!("finder.js");

/// The place a target is in the page's viewport, in points, and the corner radius of what is there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub frame: Frame,
    pub radius: f64,
}

#[derive(Deserialize)]
struct Wire {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    #[serde(default)]
    radius: f64,
}

/// The script that finds `target`, scrolls it to the middle of the viewport and answers its box as JSON.
pub fn look(target: &WalkTarget) -> String {
    ask(&format!("ArmadaFinder.finder.look({})", encoded(target)))
}

/// The script that presses the held element, as a click the page hears.
pub fn press() -> String {
    ask("ArmadaFinder.finder.press()")
}

fn ask(call: &str) -> String {
    format!("{SOURCE}\n{call}")
}

fn encoded(target: &WalkTarget) -> String {
    ipc::encode(target).unwrap_or_else(|_| "null".into())
}

/// A finder's answer as a place, bounded to the viewport. `None` where the target is not drawn (the
/// answer is `null`, empty or not a box).
pub fn place_of(answer: Option<&str>, viewport: (f64, f64)) -> Option<Place> {
    let wire: Wire = ipc::decode("finder box", answer?.as_bytes()).ok()?;
    let whole = |value: f64| if value.is_finite() { value.round() } else { 0. };
    let (x, y) = (whole(wire.x).max(0.), whole(wire.y).max(0.));
    let width = whole(wire.width).min((viewport.0 - x).max(0.));
    let height = whole(wire.height).min((viewport.1 - y).max(0.));
    (width > 0. && height > 0.).then_some(Place { frame: Frame::new(x, y, width, height), radius: whole(wire.radius).max(0.) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_is_read_and_kept_inside_the_viewport() {
        let place = place_of(Some(r#"{"x":10.4,"y":20,"width":500,"height":40,"radius":6}"#), (300., 200.)).unwrap();
        assert_eq!(place.frame, Frame::new(10., 20., 290., 40.));
        assert_eq!(place.radius, 6.);
    }

    #[test]
    fn nothing_drawn_answers_no_place() {
        assert_eq!(place_of(Some("null"), (300., 200.)), None);
        assert_eq!(place_of(None, (300., 200.)), None);
        assert_eq!(place_of(Some(r#"{"x":0,"y":0,"width":0,"height":10}"#), (300., 200.)), None);
        assert_eq!(place_of(Some(r#"{"x":400,"y":0,"width":10,"height":10}"#), (300., 200.)), None);
    }

    #[test]
    fn a_look_carries_the_target_as_the_wire_spells_it() {
        let script = look(&WalkTarget { role: "button".into(), name: Some("Add".into()), exact: None, within: None });
        assert!(script.ends_with(r#"ArmadaFinder.finder.look({"role":"button","name":"Add"})"#), "{script}");
    }
}
