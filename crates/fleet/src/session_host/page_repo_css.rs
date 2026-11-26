//! A page's kept style set as the css a sealed page carries: each kept sheet in order, after one `@font-face` for
//! each kept font with its bytes as a `data:` address, since the sealed page may fetch nothing.

use base64::Engine as _;
use ipc::{KeptFont, KeptStyles};

/// Every font face of `kept`, then every sheet, as the text of one `<style>`. `</style` is written so it cannot close it.
pub(super) fn css_of(kept: &KeptStyles) -> String {
    let faces: String = kept.fonts.iter().filter_map(font_face).collect();
    let css = format!("{faces}{}", kept.sheets.iter().map(|sheet| sheet.css.as_str()).collect::<Vec<_>>().join("\n"));
    let lower = css.to_ascii_lowercase();
    let (mut out, mut from) = (String::with_capacity(css.len()), 0);
    while let Some(at) = lower[from..].find("</style") {
        out.push_str(&css[from..from + at]);
        out.push_str("<\\/style");
        from += at + "</style".len();
    }
    out.push_str(&css[from..]);
    out
}

/// A kept font as the `@font-face` that carries its bytes. One whose bytes are not base64 or not a font is left out.
fn font_face(font: &KeptFont) -> Option<String> {
    let plain = |text: &str| !text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric() || " .%-".contains(c));
    if font.bytes.is_empty() || !font.bytes.chars().all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c)) {
        return None;
    }
    let head = base64::engine::general_purpose::STANDARD.decode(&font.bytes[..font.bytes.len().min(8) / 4 * 4]).unwrap_or_default();
    let (mime, format) = match head.get(..4)? {
        b"wOF2" => ("font/woff2", "woff2"),
        b"wOFF" => ("font/woff", "woff"),
        b"OTTO" => ("font/otf", "opentype"),
        [0, 1, 0, 0] | b"true" => ("font/ttf", "truetype"),
        _ => return None,
    };
    let family: String = font.family.chars().filter(|c| !matches!(c, '"' | '\\' | '<' | '>' | '\n' | '\r')).collect();
    let weight = if plain(&font.weight) { font.weight.as_str() } else { "400" };
    let style = font.style.as_deref().filter(|style| plain(style)).map(|style| format!("font-style:{style};")).unwrap_or_default();
    Some(format!(r#"@font-face{{font-family:"{family}";font-weight:{weight};{style}src:url(data:{mime};base64,{}) format("{format}");}}"#, font.bytes))
}
