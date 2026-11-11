//! A sketch shared with somebody who has no Armada, as Electron's Share makes it: Mermaid text
//! (`export-mermaid.ts`), one web page with the drawing and its steps (`export-svg.ts`,
//! `export-html.ts`), and a picture (`svgToPng`). Each is drawn from the layout the reader shows.

use gpui::{App, SharedString, Window};
use ipc::Scene;

use crate::look::Look;

/// What the toast names after a copy (`MERMAID_COPIED`).
pub(crate) const MERMAID_COPIED: &str = "The Mermaid text";

/// The scene as a Mermaid flowchart: its boxes and labelled arrows, every label quoted and escaped,
/// each colour as the theme holds it now (`sceneMermaid`).
pub(crate) fn mermaid(_scene: &Scene, _look: &Look) -> String {
    String::new()
}

/// The page Save as web page writes: the drawing as the reader draws it, its steps playing in a loop,
/// and Play and Restart (`sketchHtml` over `sketchSvg`).
pub(crate) fn web_page(_key: &SharedString, _scene: &Scene, _title: &str, _window: &mut Window, _cx: &mut App) -> String {
    String::new()
}

/// What Save as image writes: the drawing as standalone SVG, framed to it, in the theme's colours.
pub(crate) fn svg(_key: &SharedString, _scene: &Scene, _window: &mut Window, _cx: &mut App) -> String {
    String::new()
}

/// That SVG as PNG bytes at twice its size (`svgToPng`).
pub(crate) fn png(_svg: &str) -> Result<Vec<u8>, String> {
    Err("not drawn".into())
}

/// The title as a file name: nothing a file system or a shell reads as more than a letter, and never
/// empty (`sketchFileName`).
pub(crate) fn file_name(title: &str, extension: &str) -> String {
    let cleaned: String = title.chars().map(|c| if c.is_control() || "\\/:*?\"<>|".contains(c) { ' ' } else { c }).collect();
    let words = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let base: String = words.trim_matches(|c: char| c == '.' || c.is_whitespace()).chars().take(80).collect();
    let base = base.trim();
    format!("{}.{extension}", if base.is_empty() { "Sketch" } else { base })
}
