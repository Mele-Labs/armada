//! Words copied with their formatting: the markdown as written and the same words as HTML, on one pasteboard
//! item, so Notes, Mail, Docs or Slack keep the bold, lists, headings, links and code. gpui's `ClipboardItem`
//! carries one string, so on the Mac this writes `NSPasteboard` itself.

/// The words as HTML, read as GitHub reads markdown.
pub fn html_of(markdown: &str) -> String {
    // Only MDX can refuse; GitHub's flavour never does.
    markdown::to_html_with_options(markdown, &markdown::Options::gfm()).unwrap_or_else(|_| markdown::to_html(markdown))
}

/// Puts `markdown` on the pasteboard as plain text and as HTML in one write. A test writes gpui's own
/// clipboard, never the Mac's, which is the person's.
pub fn copy_words(markdown: &str, cx: &gpui::App) {
    #[cfg(all(target_os = "macos", not(test)))]
    {
        use objc2_app_kit::NSPasteboard;
        use objc2_foundation::NSString;
        let _ = cx;
        let board = NSPasteboard::generalPasteboard();
        board.clearContents();
        board.setString_forType(&NSString::from_str(markdown), &NSString::from_str("public.utf8-plain-text"));
        // Without the charset some apps read the HTML as Latin-1 and garble anything past ASCII.
        let html = format!("<meta charset=\"utf-8\">{}", html_of(markdown));
        board.setString_forType(&NSString::from_str(&html), &NSString::from_str("public.html"));
    }
    #[cfg(not(all(target_os = "macos", not(test))))]
    cx.write_to_clipboard(gpui::ClipboardItem::new_string(markdown.to_string()));
}

#[cfg(test)]
mod tests {
    use super::html_of;

    #[test]
    fn bold_and_a_list_come_out_as_strong_and_list_items() {
        let html = html_of("Some **bold** words.\n\n- one\n- two\n");
        assert!(html.contains("<strong>bold</strong>"), "{html}");
        assert!(html.contains("<li>one</li>") && html.contains("<li>two</li>"), "{html}");
    }
}
