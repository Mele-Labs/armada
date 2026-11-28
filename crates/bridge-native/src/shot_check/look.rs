//! `--walk <name> --check --shots <dir> --against <dir> --out <dir>`: a walk's pictures set against the
//! pictures of the look he approved, step by step, by the file name each step's picture has. Fleet
//! cannot link gpui, so this is where the two are compared; it writes `report.json` for Fleet to read.
//! A step with no approved picture, and an approved picture with no step, are both *missing*, and a
//! missing step has drifted.

use std::path::Path;

use image::RgbaImage;
use ipc::{LookReport, LookStep};

use super::{compare, write_pair};

/// The file name of a walk's step's picture, as `--shots` writes it.
pub fn file_of(walk: &str, step: usize) -> String {
    format!("{walk}-{step:02}.png")
}

/// The step an approved picture's file name is for, where it is named for `walk`.
fn step_of(walk: &str, file: &str) -> Option<usize> {
    file.strip_prefix(walk)?.strip_prefix('-')?.strip_suffix(".png")?.parse().ok()
}

fn read(path: &Path) -> Result<Option<RgbaImage>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    image::open(path).map(|picture| Some(picture.to_rgba8())).map_err(|why| format!("{}: {why}", path.display()))
}

/// The `steps` pictures of `walk` in `shots`, each set against the same file name in `against`. A drifted
/// step has its side-by-side and difference pictures written to `out`.
pub fn report(walk: &str, steps: usize, shots: &Path, against: &Path, out: &Path) -> Result<LookReport, String> {
    std::fs::create_dir_all(out).map_err(|why| format!("{}: {why}", out.display()))?;
    let mut looked = Vec::new();
    for at in 1..=steps {
        let file = file_of(walk, at);
        let (now, approved) = (read(&shots.join(&file))?, read(&against.join(&file))?);
        looked.push(match (approved, now) {
            (Some(approved), Some(now)) => {
                let score = compare::score(&approved, &now);
                let drifted = score.drifted();
                let pair = if drifted { Some(write_pair(out, file.trim_end_matches(".png"), &approved, &now)?) } else { None };
                let (side_by_side, difference) = pair.unzip();
                LookStep { step: at as u32, file, drifted, missing: false, mean: score.mean, cells: score.share, side_by_side, difference }
            }
            _ => LookStep { step: at as u32, file, drifted: true, missing: true, mean: compare::Score::APART.mean, cells: compare::Score::APART.share, side_by_side: None, difference: None },
        });
    }
    // An approved picture of a step this build no longer has.
    let mut extra: Vec<usize> = match std::fs::read_dir(against) {
        Ok(entries) => entries.filter_map(|entry| step_of(walk, &entry.ok()?.file_name().to_string_lossy())).filter(|at| *at > steps).collect(),
        Err(why) => return Err(format!("{}: {why}", against.display())),
    };
    extra.sort_unstable();
    looked.extend(extra.into_iter().map(|at| LookStep {
        step: at as u32,
        file: file_of(walk, at),
        drifted: true,
        missing: true,
        mean: compare::Score::APART.mean,
        cells: compare::Score::APART.share,
        side_by_side: None,
        difference: None,
    }));
    let drifted = looked.iter().any(|step| step.drifted);
    Ok(LookReport { walk: walk.to_string(), steps: looked, drifted })
}

/// [`report`], written to `<out>/report.json` through the wire's own codec.
pub fn write(report: &LookReport, out: &Path) -> Result<(), String> {
    let text = ipc::encode(report).map_err(|why| why.to_string())?;
    let path = out.join("report.json");
    std::fs::write(&path, text).map_err(|why| format!("{}: {why}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn page(box_at: u32) -> RgbaImage {
        let mut image = RgbaImage::from_pixel(640, 480, Rgba([30, 30, 34, 255]));
        for y in 100..160 {
            for x in box_at..box_at + 160 {
                image.put_pixel(x, y, Rgba([50, 60, 80, 255]));
            }
        }
        image
    }

    fn folder(name: &str, pictures: &[(&str, u32)]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("look-report-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (file, at) in pictures {
            page(*at).save(dir.join(file)).unwrap();
        }
        dir
    }

    #[test]
    fn an_equal_step_passes_and_a_drifted_or_missing_one_fails_the_report() {
        let approved = folder("approved", &[("w-01.png", 100), ("w-02.png", 100), ("w-04.png", 100)]);
        let shots = folder("shots", &[("w-01.png", 100), ("w-02.png", 300), ("w-03.png", 100)]);
        let out = folder("out", &[]);
        let report = report("w", 3, &shots, &approved, &out).unwrap();
        let seen: Vec<_> = report.steps.iter().map(|one| (one.step, one.drifted, one.missing)).collect();
        // 1 equal; 2 moved; 3 has no approved picture; 4 is approved with no step.
        assert_eq!(seen, vec![(1, false, false), (2, true, false), (3, true, true), (4, true, true)]);
        assert!(report.drifted);
        assert_eq!(report.steps[0].mean, 0.);
        assert!(report.steps[0].side_by_side.is_none());
        let both = report.steps[1].side_by_side.as_deref().unwrap();
        assert!(out.join(both).is_file() && out.join(report.steps[1].difference.as_deref().unwrap()).is_file());
    }

    #[test]
    fn folders_of_equal_pictures_do_not_drift_and_the_report_round_trips_the_codec() {
        let approved = folder("same-approved", &[("w-01.png", 100), ("w-02.png", 100)]);
        let shots = folder("same-shots", &[("w-01.png", 100), ("w-02.png", 101)]);
        let out = folder("same-out", &[]);
        let report = report("w", 2, &shots, &approved, &out).unwrap();
        assert!(!report.drifted, "{report:?}");
        write(&report, &out).unwrap();
        let back: LookReport = ipc::decode("LookReport", &std::fs::read(out.join("report.json")).unwrap()).unwrap();
        assert_eq!(back, report);
    }
}
