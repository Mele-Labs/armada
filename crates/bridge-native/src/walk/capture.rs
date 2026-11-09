//! Capture on a walk: the screen under a crosshair, as Electron's walk capture (`StudioCapture`).
//! The element under the pointer is outlined with its name; a press holds it, and a drag holds the
//! box drawn instead, for a spot that is not one element. The note is written in a card against
//! what was held, and is held until Send notes. Each goes with a picture of the whole window, its
//! area outlined and numbered by its place among the notes sent, as Electron burns them (`burn.ts`).

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

/// Digits three wide and five tall, a row to a `u8`, for the number burned beside a ring.
const DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b010, 0b010, 0b010],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];

/// The window's picture with `ring` outlined in `ink` and `number` in a pill at its top left, as a
/// PNG in base64. `scale` is the window's pixels per point.
pub fn ringed(picture: &image::RgbaImage, ring: Bounds<Pixels>, scale: f32, number: usize, ink: [u8; 4], on_ink: [u8; 4]) -> Option<String> {
    let mut out = picture.clone();
    let (width, height) = (out.width() as i64, out.height() as i64);
    let mut fill = |x0: i64, y0: i64, x1: i64, y1: i64, color: [u8; 4]| {
        for y in y0.max(0)..y1.min(height) {
            for x in x0.max(0)..x1.min(width) {
                out.put_pixel(x as u32, y as u32, image::Rgba(color));
            }
        }
    };
    let at = |value: Pixels| (f32::from(value) * scale).round() as i64;
    let (x0, y0) = (at(ring.origin.x), at(ring.origin.y));
    let (x1, y1) = (x0 + at(ring.size.width), y0 + at(ring.size.height));
    let edge = (2. * scale).round().max(1.) as i64;
    fill(x0, y0, x1, y0 + edge, ink);
    fill(x0, y1 - edge, x1, y1, ink);
    fill(x0, y0, x0 + edge, y1, ink);
    fill(x1 - edge, y0, x1, y1, ink);
    // The pill: each digit cell `dot` wide, padded by two cells, above the ring where it fits.
    let digits: Vec<usize> = number.to_string().bytes().map(|digit| (digit - b'0') as usize).collect();
    let dot = (3. * scale).round().max(1.) as i64;
    let (pill_w, pill_h) = (dot * (4 * digits.len() as i64 + 3), dot * 9);
    let top = if y0 - pill_h >= 0 { y0 - pill_h } else { y0 };
    fill(x0, top, x0 + pill_w, top + pill_h, ink);
    for (place, digit) in digits.iter().enumerate() {
        for (row, bits) in DIGITS[*digit].iter().enumerate() {
            for column in 0..3 {
                if bits & (0b100 >> column) != 0 {
                    let x = x0 + dot * (2 + 4 * place as i64 + column);
                    let y = top + dot * (2 + row as i64);
                    fill(x, y, x + dot, y + dot, on_ink);
                }
            }
        }
    }
    let mut png = std::io::Cursor::new(Vec::new());
    out.write_to(&mut png, image::ImageFormat::Png).ok()?;
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
    fn a_ring_is_burned_at_the_windows_scale_with_its_number_and_a_ring_past_the_edge_is_cut() {
        let picture = image::RgbaImage::new(200, 200);
        let ink = [255, 0, 0, 255];
        let ring = Bounds { origin: point(px(20.), px(40.)), size: size(px(30.), px(40.)) };
        let burned = ringed(&picture, ring, 2., 12, ink, [255; 4]).expect("a picture");
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD.decode(burned).expect("base64");
        let back = image::load_from_memory(&bytes).expect("a PNG").to_rgba8();
        assert_eq!((back.width(), back.height()), (200, 200), "the whole window, not a crop");
        assert_eq!(back.get_pixel(40, 140).0, ink, "the ring's left edge, at twice the points");
        assert_eq!(back.get_pixel(70, 120).0, [0; 4], "inside the ring is left as it was");
        assert_eq!(back.get_pixel(41, 79).0, ink, "the number's pill stands above the ring");
        assert!(ringed(&picture, Bounds { origin: point(px(90.), px(40.)), size: size(px(50.), px(50.)) }, 2., 1, ink, [255; 4]).is_some());
    }
}
