//! The rule against trees built here: each way a guide is drawn or is not.

use super::*;

/// A tree under a temporary root, so each direction of the rule is proved
/// against a state built here rather than against the one the repository is in.
struct Tree(std::path::PathBuf);

impl Tree {
    fn new(name: &str) -> Tree {
        let dir = std::env::temp_dir().join(format!("armada-guides-{name}"));
        let _ = fs::remove_dir_all(&dir);
        Tree(dir)
    }

    fn file(self, path: &str, text: &str) -> Tree {
        let at = self.0.join(path);
        fs::create_dir_all(at.parent().expect("a parent")).expect("a directory");
        fs::write(at, text).expect("a file");
        self
    }

    fn guide(self, file: &str, constant: &str, piece: &str) -> Tree {
        let text = format!(
            "export const {constant}: Guide = {{\n  number: 1,\n  piece: \"{piece}\",\n}};\n"
        );
        self.file(&format!("{GUIDES}/{file}"), &text)
    }

    fn report(&self) -> Report {
        every_guides_piece_is_drawn(&self.0)
    }
}

fn said(report: &Report) -> String {
    report
        .findings
        .iter()
        .map(|f| match f {
            crate::Finding::Fail(s) | crate::Finding::Warn(s) => s.as_str(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_guide_no_screen_draws_fails() {
    let report = Tree::new("undrawn")
        .guide("020-drift.ts", "GUIDE_DRIFT", "run.drift")
        .file("packages/screens/src/Other.tsx", "export const X = 1;\n")
        .report();
    assert!(report.failed());
    let said = said(&report);
    assert!(
        said.contains("run.drift") && said.contains("GUIDE_DRIFT"),
        "{said}"
    );
}

#[test]
fn a_guide_a_screen_marks_passes() {
    let report = Tree::new("drawn")
        .guide("009-looks.ts", "GUIDE_ALWAYS_LOOKS", "run.always-looks")
        .file(
            "packages/screens/src/Run.tsx",
            "<GuideMark guide={GUIDE_ALWAYS_LOOKS} />\n",
        )
        .report();
    assert!(!report.failed(), "{}", said(&report));
}

#[test]
fn the_five_catalogue_only_pieces_need_no_mark() {
    let mut tree = Tree::new("exempt");
    for (n, piece) in CATALOGUE_ONLY.iter().enumerate() {
        tree = tree.guide(&format!("{:03}-x.ts", n + 1), &format!("GUIDE_{n}"), piece);
    }
    assert!(!tree.report().failed());
}

#[test]
fn a_mark_only_in_a_story_or_a_test_does_not_count() {
    let report = Tree::new("story-only")
        .guide("008-bar.ts", "GUIDE_STEP_BAR", "run.step-bar")
        .file(
            "packages/components/src/X/X.stories.tsx",
            "<GuideMark guide={GUIDE_STEP_BAR} />\n",
        )
        .file("packages/screens/src/Y.test.tsx", "GUIDE_STEP_BAR;\n")
        .report();
    assert!(report.failed());
}

#[test]
fn the_catalogue_index_naming_a_guide_does_not_count() {
    let report = Tree::new("index-only")
        .guide("008-bar.ts", "GUIDE_STEP_BAR", "run.step-bar")
        .file(
            &format!("{GUIDES}/index.ts"),
            "export { GUIDE_STEP_BAR };\n",
        )
        .report();
    assert!(report.failed());
}

#[test]
fn a_longer_name_is_not_the_guide() {
    let report = Tree::new("prefix")
        .guide("017-plan.ts", "GUIDE_PLAN_X", "plan.x")
        .file(
            "apps/desktop/src/A.tsx",
            "GUIDE_PLAN_XY; MY_GUIDE_PLAN_X;\n",
        )
        .report();
    assert!(report.failed());
}

#[test]
fn a_guide_file_of_another_shape_is_named() {
    let report = Tree::new("shape")
        .file(&format!("{GUIDES}/001-odd.ts"), "export default {};\n")
        .report();
    assert!(report.failed());
}

#[test]
fn a_missing_catalogue_fails() {
    assert!(Tree::new("missing").report().failed());
}
