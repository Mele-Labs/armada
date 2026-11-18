//! How wide the Files rail is drawn, held between a least and a most and kept in one small file on this
//! Mac, so it is the same on every pull request and after a restart.

use std::path::PathBuf;

/// The width the rail starts at where none is kept, in logical pixels.
pub(super) const DEFAULT: f32 = 200.;
/// The least the rail is dragged to, so its names stay a few words wide.
pub(super) const LEAST: f32 = 140.;
/// The most the rail is dragged to, so the panes keep room.
pub(super) const MOST: f32 = 560.;

/// A width held to the least and most; a number that is not one reads as the default.
pub(super) fn clamp(width: f32) -> f32 {
    if width.is_finite() { width.clamp(LEAST, MOST) } else { DEFAULT }
}

/// The width `text` keeps, or None where it is not a number.
pub(super) fn parse(text: &str) -> Option<f32> {
    text.trim().parse::<f32>().ok().filter(|width| width.is_finite()).map(clamp)
}

/// The file the width is kept in: beside the Jobs list's own files.
fn file() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name("reviews-rail-width"))
}

/// The width kept on this Mac, or the default.
pub(super) fn load() -> f32 {
    file().and_then(|path| std::fs::read_to_string(path).ok()).and_then(|text| parse(&text)).unwrap_or(DEFAULT)
}

/// Keeps `width`. Written whole to a file beside it and moved into place, so a crash leaves the last one.
pub(super) fn keep(width: f32) {
    let Some(path) = file() else { return };
    let fresh = path.with_extension("new");
    let done = std::fs::write(&fresh, format!("{}", clamp(width))).and_then(|()| std::fs::rename(&fresh, &path));
    if let Err(why) = done {
        crate::log::Note::warn("the Files rail's width was not kept").field("why", why.to_string()).emit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_width_is_held_between_the_least_and_the_most() {
        assert_eq!(clamp(10.), LEAST);
        assert_eq!(clamp(9000.), MOST);
        assert_eq!(clamp(300.), 300.);
        assert_eq!(clamp(f32::NAN), DEFAULT);
    }

    #[test]
    fn a_kept_width_reads_back_and_junk_does_not() {
        assert_eq!(parse("312.5\n"), Some(312.5));
        assert_eq!(parse("50"), Some(LEAST));
        assert_eq!(parse("wide"), None);
        assert_eq!(parse("inf"), None);
    }
}
