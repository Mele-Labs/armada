//! Capture on a walk: the screen under a crosshair, as Electron's walk capture (`StudioCapture`).
//! The element under the pointer is outlined with its name; a press holds it, and a drag holds the
//! box drawn instead, for a spot that is not one element. The note is written in a card against
//! what was held, and goes with a picture of it.

use gpui::{Bounds, Pixels, Point, Role, SharedString};

use crate::labels::Drawn;

/// Below this, in each direction, a press that moved is still a press and not a box.
pub const SLOP: f32 = 4.;

/// What a note is written on.
#[derive(Clone, Debug)]
pub enum Held {
    Element(Drawn),
    Region(Bounds<Pixels>),
}

impl Held {
    pub fn bounds(&self) -> Bounds<Pixels> {
        match self {
            Held::Element(drawn) => drawn.bounds,
            Held::Region(bounds) => *bounds,
        }
    }

    /// The words the card heads with, and the note's place carries: the element's role and name,
    /// or that it is a box.
    pub fn said(&self) -> SharedString {
        match self {
            Held::Element(drawn) => match role_word(drawn.role) {
                Some(role) => format!("{role} · {}", drawn.name).into(),
                None => drawn.name.clone(),
            },
            Held::Region(_) => "A box you drew".into(),
        }
    }
}

/// A role in the words a walk step uses for it.
pub fn role_word(role: Role) -> Option<&'static str> {
    Some(match role {
        Role::Button => "Button",
        Role::Row => "Row",
        Role::List => "List",
        Role::ListItem => "List item",
        Role::Heading => "Heading",
        Role::TextInput | Role::SearchInput => "Field",
        Role::Toolbar => "Toolbar",
        Role::Status => "Status",
        Role::Link => "Link",
        Role::Menu => "Menu",
        Role::MenuItem | Role::MenuItemCheckBox => "Menu item",
        Role::Image => "Mark",
        _ => return None,
    })
}

/// The labelled element of `drawn` under `at` and inside `within`: the smallest that holds the
/// point, so a row wins over its list.
pub fn element_at(drawn: &[Drawn], at: Point<Pixels>, within: Bounds<Pixels>) -> Option<Drawn> {
    drawn
        .iter()
        .filter(|one| one.bounds.contains(&at) && within.contains(&one.bounds.center()))
        .min_by(|a, b| area(a.bounds).total_cmp(&area(b.bounds)))
        .cloned()
}

fn area(bounds: Bounds<Pixels>) -> f32 {
    f32::from(bounds.size.width) * f32::from(bounds.size.height)
}

/// The box from `from` to `to`, whichever way it was dragged, or `None` for a press that barely moved.
pub fn region(from: Point<Pixels>, to: Point<Pixels>) -> Option<Bounds<Pixels>> {
    let (x, y) = ((f32::from(to.x) - f32::from(from.x)).abs(), (f32::from(to.y) - f32::from(from.y)).abs());
    if x < SLOP && y < SLOP {
        return None;
    }
    let origin = Point { x: from.x.min(to.x), y: from.y.min(to.y) };
    Some(Bounds { origin, size: gpui::size(gpui::px(x), gpui::px(y)) })
}

/// `bounds` cut out of the window's picture, as a PNG in base64. `scale` is the window's pixels per point.
pub fn crop(picture: &image::RgbaImage, bounds: Bounds<Pixels>, scale: f32) -> Option<String> {
    let at = |value: Pixels| (f32::from(value) * scale).max(0.) as u32;
    let (x, y) = (at(bounds.origin.x).min(picture.width()), at(bounds.origin.y).min(picture.height()));
    let width = at(bounds.size.width).min(picture.width() - x);
    let height = at(bounds.size.height).min(picture.height() - y);
    if width == 0 || height == 0 {
        return None;
    }
    let cut = image::imageops::crop_imm(picture, x, y, width, height).to_image();
    let mut png = std::io::Cursor::new(Vec::new());
    cut.write_to(&mut png, image::ImageFormat::Png).ok()?;
    use base64::Engine as _;
    Some(base64::engine::general_purpose::STANDARD.encode(png.into_inner()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px, size};

    fn drawn(name: &str, role: Role, x: f32, y: f32, w: f32, h: f32) -> Drawn {
        Drawn { role, name: name.to_string().into(), bounds: Bounds { origin: point(px(x), px(y)), size: size(px(w), px(h)) } }
    }

    #[test]
    fn the_smallest_element_under_the_pointer_is_the_one_held() {
        let screen = Bounds { origin: point(px(0.), px(100.)), size: size(px(800.), px(600.)) };
        let all = vec![
            drawn("Jobs", Role::List, 10., 110., 300., 500.),
            drawn("Audit the docs index", Role::Row, 20., 200., 280., 36.),
            drawn("Walk", Role::Toolbar, 0., 0., 800., 88.),
        ];
        assert_eq!(element_at(&all, point(px(50.), px(210.)), screen).map(|one| one.name), Some("Audit the docs index".into()));
        assert_eq!(element_at(&all, point(px(50.), px(500.)), screen).map(|one| one.name), Some("Jobs".into()));
        // The walk bar is not the screen.
        assert!(element_at(&all, point(px(50.), px(40.)), screen).is_none());
    }

    #[test]
    fn a_drag_past_the_slop_is_a_box_either_way_and_a_press_is_not() {
        assert!(region(point(px(10.), px(10.)), point(px(12.), px(13.))).is_none());
        let drawn = region(point(px(100.), px(80.)), point(px(40.), px(20.))).expect("a box");
        assert_eq!(drawn.origin, point(px(40.), px(20.)));
        assert_eq!(drawn.size, size(px(60.), px(60.)));
    }

    #[test]
    fn a_crop_is_cut_at_the_windows_scale_and_kept_inside_it() {
        let picture = image::RgbaImage::new(200, 100);
        let inside = Bounds { origin: point(px(10.), px(10.)), size: size(px(20.), px(20.)) };
        assert!(crop(&picture, inside, 2.).is_some());
        let outside = Bounds { origin: point(px(500.), px(500.)), size: size(px(20.), px(20.)) };
        assert!(crop(&picture, outside, 2.).is_none());
    }
}
