//! What a Job browser's page may do besides ask: open a popup, download a file, show a dialog, as the owner's choices say
//! (`main/job-browsers.ts`'s window open handler, download decider and will-navigate). The decisions are here, without WebKit;
//! `wk.rs` carries them out. `Way` is the owner's choice for the page's host, handed in by the tab that holds the page.

use std::path::{Path, PathBuf};

/// The owner's choice for what a page asks to do: Allow, Ask or Block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Way {
    Allow,
    #[default]
    Ask,
    Block,
}

/// Whether an address may be loaded by a page's popup, or by the page itself: only the web (`webOrigin`). A popup's first address is
/// often blank, so `about:blank` is let through here for the one that opens before its own address arrives.
pub fn only_web(address: &str) -> bool {
    let lower = address.trim().to_ascii_lowercase();
    ["http://", "https://"].iter().any(|scheme| lower.strip_prefix(scheme).is_some_and(|rest| !rest.starts_with(['/', '?', '#'])))
}

pub fn is_blank(address: &str) -> bool {
    address.is_empty() || address == "about:blank"
}

/// What a page that asks to open a window is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupWay {
    /// A window of its own, made from the page's own configuration, so `window.opener` works for sign-in flows.
    Open,
    /// The owner is asked, and a window with the address is opened after his Allow (without `window.opener`, as Classic does).
    Ask,
    Refuse,
}

/// The way a page's popup goes, by what it wants to open and what the owner chose for the page that opens it (`setWindowOpenHandler`).
/// Only a web address opens, and a page with no host of its own opens nothing.
pub fn popup_way(opener_host: Option<&str>, wants: &str, way: Way) -> PopupWay {
    if opener_host.is_none() || !(only_web(wants) || is_blank(wants)) {
        return PopupWay::Refuse;
    }
    match way {
        Way::Block => PopupWay::Refuse,
        // A blank window has no address to ask about.
        Way::Ask if is_blank(wants) => PopupWay::Refuse,
        Way::Ask => PopupWay::Ask,
        Way::Allow => PopupWay::Open,
    }
}

/// What a download is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownloadWay {
    Take,
    /// Held back, the owner asked, and taken after his Allow.
    Ask,
    Refuse,
}

/// The way a download goes, by the host it comes from (`download`): a host that is no web address is blocked.
pub fn download_way(host: Option<&str>, way: Way) -> DownloadWay {
    match (host, way) {
        (None, _) | (_, Way::Block) => DownloadWay::Refuse,
        (Some(_), Way::Ask) => DownloadWay::Ask,
        (Some(_), Way::Allow) => DownloadWay::Take,
    }
}

/// Whether a response is a file the page cannot show, so it is downloaded: its type is none a web view draws, or the server said to save it.
pub fn is_download(can_show: bool, content_disposition: Option<&str>) -> bool {
    let attachment = content_disposition.is_some_and(|said| said.trim_start().to_ascii_lowercase().starts_with("attachment"));
    !can_show || attachment
}

/// A file name a server suggested, made safe to write under the downloads folder: no folder in it, no leading dots, never empty.
pub fn safe_name(suggested: &str) -> String {
    let last = suggested.rsplit(['/', '\\']).next().unwrap_or_default();
    let cleaned: String = last.chars().filter(|c| !c.is_control() && *c != ':').collect();
    let cleaned = cleaned.trim().trim_start_matches('.').to_string();
    if cleaned.is_empty() {
        "download".to_string()
    } else {
        cleaned
    }
}

/// Where a download is written: in `folder`, under `suggested`'s name, or under it with a number where a file has that name already
/// (`report.pdf`, `report (1).pdf`, `report (2).pdf`).
pub fn unique_path(folder: &Path, suggested: &str, taken: impl Fn(&Path) -> bool) -> PathBuf {
    let name = safe_name(suggested);
    let first = folder.join(&name);
    if !taken(&first) {
        return first;
    }
    let path = Path::new(&name);
    let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or(&name).to_string();
    let extension = path.extension().and_then(|extension| extension.to_str()).map(|extension| format!(".{extension}")).unwrap_or_default();
    (1..)
        .map(|number| folder.join(format!("{stem} ({number}){extension}")))
        .find(|candidate| !taken(candidate))
        .expect("an unbounded count finds a free name")
}

/// What a page's dialogs are answered where the page is in no window to show one on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialog {
    Alert,
    Confirm,
    Prompt,
}

/// What a page that is in no window is answered: an alert is dismissed, a confirm is answered false, and a prompt is answered with
/// nothing, as it is in a window too (`prompt` is not supported, as in Classic).
pub fn unplaced_answer(dialog: Dialog) -> Option<bool> {
    match dialog {
        Dialog::Alert => Some(true),
        Dialog::Confirm => Some(false),
        Dialog::Prompt => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_web_is_loaded() {
        assert!(only_web("https://example.com/a"));
        assert!(only_web("http://localhost:3000"));
        for no in ["file:///etc/passwd", "javascript:alert(1)", "data:text/html,hi", "mailto:a@b.c", "about:blank", "armada://x", ""] {
            assert!(!only_web(no), "{no}");
        }
        assert!(is_blank("about:blank") && is_blank(""));
    }

    #[test]
    fn a_popup_goes_by_the_choice_and_only_for_the_web_from_a_page_with_a_host() {
        assert_eq!(popup_way(Some("a.com"), "https://b.com/x", Way::Allow), PopupWay::Open);
        assert_eq!(popup_way(Some("a.com"), "about:blank", Way::Allow), PopupWay::Open, "a sign-in window opens blank first");
        assert_eq!(popup_way(Some("a.com"), "https://b.com/x", Way::Ask), PopupWay::Ask);
        assert_eq!(popup_way(Some("a.com"), "about:blank", Way::Ask), PopupWay::Refuse, "nothing to ask about");
        assert_eq!(popup_way(Some("a.com"), "https://b.com/x", Way::Block), PopupWay::Refuse);
        assert_eq!(popup_way(Some("a.com"), "file:///x", Way::Allow), PopupWay::Refuse);
        assert_eq!(popup_way(Some("a.com"), "javascript:1", Way::Allow), PopupWay::Refuse);
        assert_eq!(popup_way(None, "https://b.com/", Way::Allow), PopupWay::Refuse, "no opener host, no window");
    }

    #[test]
    fn a_download_goes_by_the_choice_and_a_host_that_is_no_web_address_is_blocked() {
        assert_eq!(download_way(Some("a.com"), Way::Allow), DownloadWay::Take);
        assert_eq!(download_way(Some("a.com"), Way::Ask), DownloadWay::Ask);
        assert_eq!(download_way(Some("a.com"), Way::Block), DownloadWay::Refuse);
        assert_eq!(download_way(None, Way::Allow), DownloadWay::Refuse);
    }

    #[test]
    fn a_response_is_a_download_when_the_page_cannot_show_it_or_the_server_says_to_save_it() {
        assert!(!is_download(true, None));
        assert!(is_download(false, None));
        assert!(is_download(true, Some("attachment; filename=\"a.pdf\"")));
        assert!(is_download(true, Some("  Attachment")));
        assert!(!is_download(true, Some("inline")));
    }

    #[test]
    fn a_suggested_name_is_kept_inside_the_folder() {
        assert_eq!(safe_name("report.pdf"), "report.pdf");
        assert_eq!(safe_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_name("C:\\x\\y.txt"), "y.txt");
        assert_eq!(safe_name(".hidden"), "hidden");
        assert_eq!(safe_name("a:b"), "ab");
        assert_eq!(safe_name(""), "download");
        assert_eq!(safe_name("../"), "download");
    }

    #[test]
    fn a_taken_name_gets_a_number_before_its_extension() {
        let folder = Path::new("/Users/x/Downloads");
        let taken = |names: &'static [&'static str]| move |path: &Path| names.iter().any(|name| folder.join(name) == path);
        assert_eq!(unique_path(folder, "a.pdf", taken(&[])), folder.join("a.pdf"));
        assert_eq!(unique_path(folder, "a.pdf", taken(&["a.pdf"])), folder.join("a (1).pdf"));
        assert_eq!(unique_path(folder, "a.pdf", taken(&["a.pdf", "a (1).pdf", "a (2).pdf"])), folder.join("a (3).pdf"));
        assert_eq!(unique_path(folder, "notes", taken(&["notes"])), folder.join("notes (1)"));
        assert_eq!(unique_path(folder, "a.tar.gz", taken(&["a.tar.gz"])), folder.join("a.tar (1).gz"));
    }

    #[test]
    fn a_page_in_no_window_has_its_alert_dismissed_its_confirm_refused_and_its_prompt_answered_with_nothing() {
        assert_eq!(unplaced_answer(Dialog::Alert), Some(true));
        assert_eq!(unplaced_answer(Dialog::Confirm), Some(false));
        assert_eq!(unplaced_answer(Dialog::Prompt), None);
    }
}
