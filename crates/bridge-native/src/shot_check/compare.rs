//! Two pictures of one screen, compared as a person's eye would: each is averaged down to a coarse
//! grid, so antialiasing and a one-pixel shift of a part wash out, and a changed colour, a moved
//! part or a missing one does not. The limits are measured, not guessed: `docs/spikes/scene-draw-time.md`,
//! *Shot check tolerance*.

use image::{imageops, Rgba, RgbaImage};

/// Cells across a picture. A cell is a square, so the rows follow from the height.
pub const GRID_COLS: u32 = 32;
/// A cell differs when its mean colour is this far from the other picture's, as a share of full scale.
pub const CELL_DIFF: f32 = 0.02;
/// A pair has drifted when the mean difference over all cells passes this.
pub const MEAN_LIMIT: f32 = 0.003;
/// Or when this share of the cells differ.
pub const SHARE_LIMIT: f32 = 0.005;
/// Two pictures further apart in size than this many pixels are not the same screen.
pub const SIZE_SLACK: u32 = 2;

/// How far apart two pictures are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Score {
    /// The mean colour difference over the grid, from 0 to 1.
    pub mean: f32,
    /// The share of cells that differ by more than [`CELL_DIFF`].
    pub share: f32,
}

impl Score {
    /// Past either limit.
    pub fn drifted(&self) -> bool {
        self.mean > MEAN_LIMIT || self.share > SHARE_LIMIT
    }

    /// What two pictures of different sizes score.
    pub const APART: Score = Score { mean: 1., share: 1. };
}

/// The two pictures cut to the corner they share, or none when their sizes are too far apart to be one screen.
fn shared(a: &RgbaImage, b: &RgbaImage) -> Option<(RgbaImage, RgbaImage)> {
    if a.width().abs_diff(b.width()) > SIZE_SLACK || a.height().abs_diff(b.height()) > SIZE_SLACK {
        return None;
    }
    let (w, h) = (a.width().min(b.width()), a.height().min(b.height()));
    Some((imageops::crop_imm(a, 0, 0, w, h).to_image(), imageops::crop_imm(b, 0, 0, w, h).to_image()))
}

/// Each cell's mean colour, row by row.
fn grid(image: &RgbaImage) -> Vec<[f32; 3]> {
    let cell = image.width().div_ceil(GRID_COLS).max(1);
    let (cols, rows) = (image.width().div_ceil(cell), image.height().div_ceil(cell));
    let mut cells = Vec::with_capacity((cols * rows) as usize);
    for row in 0..rows {
        for col in 0..cols {
            let (mut sum, mut seen) = ([0f32; 3], 0f32);
            for y in row * cell..((row + 1) * cell).min(image.height()) {
                for x in col * cell..((col + 1) * cell).min(image.width()) {
                    let Rgba([r, g, b, _]) = *image.get_pixel(x, y);
                    sum[0] += f32::from(r);
                    sum[1] += f32::from(g);
                    sum[2] += f32::from(b);
                    seen += 1.;
                }
            }
            cells.push(sum.map(|channel| channel / seen.max(1.) / 255.));
        }
    }
    cells
}

/// How far apart `a` and `b` are.
pub fn score(a: &RgbaImage, b: &RgbaImage) -> Score {
    let Some((a, b)) = shared(a, b) else {
        return Score::APART;
    };
    let (a, b) = (grid(&a), grid(&b));
    let apart: Vec<f32> = a.iter().zip(&b).map(|(x, y)| (0..3).map(|c| (x[c] - y[c]).abs()).sum::<f32>() / 3.).collect();
    let count = apart.len().max(1) as f32;
    Score { mean: apart.iter().sum::<f32>() / count, share: apart.iter().filter(|gap| **gap > CELL_DIFF).count() as f32 / count }
}

/// `scene` on the left and `built` on the right, a gap of white between.
pub fn side_by_side(scene: &RgbaImage, built: &RgbaImage) -> RgbaImage {
    let gap = 16;
    let mut both = RgbaImage::from_pixel(scene.width() + gap + built.width(), scene.height().max(built.height()), Rgba([255, 255, 255, 255]));
    imageops::replace(&mut both, scene, 0, 0);
    imageops::replace(&mut both, built, i64::from(scene.width() + gap), 0);
    both
}

/// Where the two differ, pixel by pixel: black where they agree, brighter the further apart.
/// Pictures too far apart in size to compare draw as white.
pub fn difference(a: &RgbaImage, b: &RgbaImage) -> RgbaImage {
    let Some((a, b)) = shared(a, b) else {
        return RgbaImage::from_pixel(a.width().max(b.width()), a.height().max(b.height()), Rgba([255, 255, 255, 255]));
    };
    let mut out = RgbaImage::from_pixel(a.width(), a.height(), Rgba([0, 0, 0, 255]));
    for (x, y, Rgba(p)) in a.enumerate_pixels() {
        let Rgba(q) = *b.get_pixel(x, y);
        let gap = (0..3).map(|c| u32::from(p[c].abs_diff(q[c]))).max().unwrap_or(0);
        let shown = (gap * 4).min(255) as u8;
        out.put_pixel(x, y, Rgba([shown, shown, shown, 255]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page of one colour with a box on it, as a screen has a part.
    fn page(box_at: (u32, u32), box_color: [u8; 3]) -> RgbaImage {
        let mut image = RgbaImage::from_pixel(640, 480, Rgba([30, 30, 34, 255]));
        for y in box_at.1..box_at.1 + 60 {
            for x in box_at.0..box_at.0 + 160 {
                image.put_pixel(x, y, Rgba([box_color[0], box_color[1], box_color[2], 255]));
            }
        }
        image
    }

    #[test]
    fn a_picture_matches_itself() {
        let one = page((100, 100), [50, 60, 80]);
        let score = score(&one, &one);
        assert!(!score.drifted(), "{score:?}");
        assert_eq!(score.mean, 0.);
    }

    #[test]
    fn a_one_pixel_shift_still_matches() {
        let score = score(&page((100, 100), [50, 60, 80]), &page((101, 100), [50, 60, 80]));
        assert!(!score.drifted(), "{score:?}");
    }

    #[test]
    fn a_part_moved_across_the_screen_drifts() {
        let score = score(&page((100, 100), [50, 60, 80]), &page((300, 300), [50, 60, 80]));
        assert!(score.drifted(), "{score:?}");
    }

    #[test]
    fn a_part_that_changes_colour_drifts() {
        let score = score(&page((100, 100), [50, 60, 80]), &page((100, 100), [80, 60, 50]));
        assert!(score.drifted(), "{score:?}");
    }

    #[test]
    fn pictures_of_different_sizes_are_apart() {
        let wide = RgbaImage::from_pixel(700, 480, Rgba([30, 30, 34, 255]));
        assert!(score(&page((0, 0), [0, 0, 0]), &wide).drifted());
    }

    #[test]
    fn the_pair_is_drawn_side_by_side_and_the_difference_is_black_where_they_agree() {
        let (one, two) = (page((100, 100), [50, 60, 80]), page((300, 300), [50, 60, 80]));
        let both = side_by_side(&one, &two);
        assert_eq!((both.width(), both.height()), (640 + 16 + 640, 480));
        let gap = difference(&one, &two);
        assert_eq!(*gap.get_pixel(0, 0), Rgba([0, 0, 0, 255]));
        assert_ne!(*gap.get_pixel(150, 120), Rgba([0, 0, 0, 255]));
    }
}
