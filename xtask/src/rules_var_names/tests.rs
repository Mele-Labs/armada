//! The rule against a surface built here, so each defect can be written in and
//! taken out again. The three names are the ones from #1668's table.

use super::*;
use crate::Finding;

/// A token set and a surface under a temporary root.
struct Surface {
    root: std::path::PathBuf,
}

impl Surface {
    fn new(name: &str) -> Surface {
        let root = std::env::temp_dir().join(format!("armada-var-names-{name}"));
        let _ = fs::remove_dir_all(&root);
        let surface = Surface { root };
        surface
            .file(
                "packages/tokens/src/spacing.css",
                ":root {\n  --space-4: 16px;\n  --space-6: 24px;\n}\n",
            )
            .file(
                "packages/tokens/src/typography.css",
                ":root {\n  --tracking-caps: 0.04em;\n  --fg-muted: grey;\n}\n",
            )
            .file(
                "packages/tokens/src/semantic.css",
                ":root {\n  --text-label: var(--fg-muted);\n  --status-running: blue;\n}\n",
            )
    }

    fn file(self, rel: &str, text: &str) -> Surface {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        fs::write(&path, text).expect("a file");
        self
    }

    fn run(self) -> Vec<String> {
        let report = every_var_names_a_declared_property(&self.root);
        let _ = fs::remove_dir_all(&self.root);
        report
            .findings
            .iter()
            .map(|f| match f {
                Finding::Fail(what) | Finding::Warn(what) => what.clone(),
            })
            .collect()
    }
}

const CARD: &str = "packages/components/src/compositions/Card/Card.css";

#[test]
fn a_rung_not_on_the_ladder_is_refused_and_told_the_ladder() {
    let lines = Surface::new("space-5")
        .file(CARD, ".armada-card {\n  gap: var(--space-5);\n}\n")
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with(&format!("{CARD}:2 — `var(--space-5)`")),
        "{lines:?}"
    );
    assert!(lines[0].contains("--space-4, --space-6"), "{lines:?}");
}

#[test]
fn a_token_that_never_existed_is_refused() {
    let lines = Surface::new("tracking")
        .file(
            CARD,
            ".armada-card { letter-spacing: var(--tracking-label); }\n",
        )
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("`var(--tracking-label)`"), "{lines:?}");
}

/// The third row, held on purpose: a real colour token used as a size is out
/// of this rule's reach, and a change that catches it is a different rule.
#[test]
fn a_real_token_in_the_wrong_role_is_not_this_rules_finding() {
    let lines = Surface::new("text-label")
        .file(CARD, ".armada-card { font-size: var(--text-label); }\n")
        .run();
    assert!(lines.is_empty(), "{lines:?}");
}

/// `--armada-stat-hue`, set by the component's style object and read back by
/// its stylesheet, as the Stats panel does — and the same set by `setProperty`.
#[test]
fn a_property_a_component_sets_on_itself_resolves() {
    let lines = Surface::new("local")
        .file(
            "packages/components/src/compositions/Stats/Stats.tsx",
            "const hue = { \"--armada-stat-hue\": `var(--${row.hue})` };\n",
        )
        .file(
            "packages/components/src/primitives/Tabs/fill.ts",
            "shape.style.setProperty(\"--armada-tab-fill-w\", w);\n",
        )
        .file(
            CARD,
            ".armada-card { color: var(--armada-stat-hue); width: var(--armada-tab-fill-w, 0); }\n",
        )
        .run();
    assert!(lines.is_empty(), "{lines:?}");
}

#[test]
fn a_name_nothing_sets_is_refused_even_with_a_fallback() {
    let lines = Surface::new("fallback")
        .file(
            CARD,
            ".armada-card { width: var(--armada-card-w, var(--space-4)); }\n",
        )
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("`var(--armada-card-w)`"), "{lines:?}");
    assert!(
        lines[0].contains("only its fallback is ever read"),
        "{lines:?}"
    );
}

/// An inline style in the app is read as well as a package's stylesheet, and
/// a stem built from a status resolves only if some status starts with it.
#[test]
fn an_inline_style_and_a_stem_are_read_too() {
    let lines = Surface::new("inline")
        .file(
            "apps/desktop/src/renderer/Board.tsx",
            concat!(
                "<div style={{ gap: \"var(--space-5)\" }} />\n",
                "const a = `var(--status-${s})`;\n",
                "const b = `var(--stauts-${s})`;\n",
            ),
        )
        .run();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(
        lines[0].contains("Board.tsx:1 — `var(--space-5)`"),
        "{lines:?}"
    );
    assert!(
        lines[1].contains("Board.tsx:3 — `var(--stauts-${…})`"),
        "{lines:?}"
    );
}

/// A comment naming a missing token is prose, and a BEM modifier ending in
/// `:hover` declares nothing.
#[test]
fn comments_and_modifiers_are_not_reads_or_declarations() {
    let lines = Surface::new("comments")
        .file(
            CARD,
            concat!(
                "/* not var(--space-5) */\n",
                ".armada-card--active:hover { color: var(--active); }\n",
            ),
        )
        .file(
            "packages/components/src/compositions/Card/Card.tsx",
            "// was var(--space-5)\nconst x = 1;\n",
        )
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains(":2 — `var(--active)`"), "{lines:?}");
}
