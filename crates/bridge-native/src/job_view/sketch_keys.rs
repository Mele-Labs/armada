//! The keys that move the view of a sketch read over the window, as pure moves of its lens: `+`, `-`, `0` and the arrows, and
//! ⌘= ⌘- ⌘0, which are the zoom keys the Electron panel binds. Kept apart from the sheet so a test can press them.

use crate::scene::Lens;

/// One key's zoom step and pan, and the most and least the reader may zoom past the fit.
pub(super) const KEY_STEP: f32 = 1.5;
pub(super) const KEY_PAN: f32 = 64.;
pub(super) const MOST: f32 = 4.;
pub(super) const LEAST: f32 = 0.5;

/// How the view of the sketch moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Looking {
    Zoom(f32),
    Fit,
    Pan(f32, f32),
}

/// The lens after a move: zoom is a factor kept between `LEAST` and `MOST`, pan moves the drawing, fit puts both back.
pub(super) fn moved(lens: Lens, by: Looking) -> Lens {
    match by {
        Looking::Zoom(by) => Lens { zoom: (lens.zoom * by).clamp(LEAST, MOST), ..lens },
        Looking::Pan(x, y) => Lens { pan: (lens.pan.0 + x, lens.pan.1 + y), ..lens },
        Looking::Fit => Lens::default(),
    }
}

/// What a key does to the view. ⌘ with `=`, `+`, `-` or `0` zooms and fits; the bare keys do too, and the arrows move it. A
/// chord with control or ⌥ is not the view's.
pub(super) fn lens_key(key: &gpui::Keystroke) -> Option<Looking> {
    let modifiers = key.modifiers;
    if modifiers.control || modifiers.alt || modifiers.function {
        return None;
    }
    let zoom = |name: &str| match name {
        "+" | "=" => Some(Looking::Zoom(KEY_STEP)),
        "-" => Some(Looking::Zoom(1. / KEY_STEP)),
        "0" => Some(Looking::Fit),
        _ => None,
    };
    if modifiers.platform {
        return zoom(key.key.as_str());
    }
    match key.key.as_str() {
        "left" => Some(Looking::Pan(KEY_PAN, 0.)),
        "right" => Some(Looking::Pan(-KEY_PAN, 0.)),
        "up" => Some(Looking::Pan(0., KEY_PAN)),
        "down" => Some(Looking::Pan(0., -KEY_PAN)),
        name => zoom(name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pressed(keys: &[&'static str]) -> Lens {
        keys.iter().fold(Lens::default(), |lens, key| {
            let key = gpui::Keystroke::parse(key).expect("a key");
            lens_key(&key).map_or(lens, |by| moved(lens, by))
        })
    }

    #[test]
    fn the_zoom_keys_with_command_move_the_zoom() {
        assert_eq!(pressed(&["cmd-="]).zoom, KEY_STEP);
        assert_eq!(pressed(&["cmd-=", "cmd-="]).zoom, KEY_STEP * KEY_STEP);
        assert_eq!(pressed(&["cmd-=", "cmd--"]).zoom, 1.);
        assert!(pressed(&["cmd--"]).zoom < 1.);
        assert_eq!(pressed(&["cmd-=", "cmd-=", "cmd-0"]), Lens::default(), "command zero fits it again");
    }

    #[test]
    fn the_bare_keys_zoom_and_the_arrows_move_the_drawing() {
        assert_eq!(pressed(&["+"]).zoom, KEY_STEP);
        assert_eq!(pressed(&["="]).zoom, KEY_STEP);
        assert_eq!(pressed(&["left"]).pan, (KEY_PAN, 0.));
        assert_eq!(pressed(&["left", "down"]).pan, (KEY_PAN, -KEY_PAN));
    }

    #[test]
    fn the_zoom_stays_between_its_least_and_its_most() {
        assert_eq!(pressed(&["cmd-="; 12]).zoom, MOST);
        assert_eq!(pressed(&["cmd--"; 12]).zoom, LEAST);
    }

    #[test]
    fn a_control_or_option_chord_is_not_the_views() {
        assert_eq!(pressed(&["ctrl-=", "alt--", "ctrl-cmd-0"]), Lens::default());
    }
}
