//! Web pages in the native Bridge, drawn by Apple's web view (WKWebView) as Armada Classic draws them with a
//! live `WebContentsView` laid over the empty frame its pane reports. **A page is a real NSView**, so the
//! same shape ports directly: the element in `page` reports its bounds each frame, the page is placed there as
//! a subview of the gpui window's own view, hidden when the element is not drawn, and a picture of the page
//! stands in its place while a gpui layer covers it. The owner chose this over embedding Chrome (13 Nov).
//!
//! | Piece | Where |
//! |---|---|
//! | The page and its element | [`WebPage`], [`web_pane`] |
//! | One picture of an HTML string, in a hidden view | [`render_html`] |
//! | Which keys go back to Bridge | [`keys`] |
//! | A sealed page's kit: its posts, the Fleet calls they make, and the answers handed back | [`kit`] |
//! | Walk steps found in a page, and ringed | [`finder`] |
//! | The WKWebView itself, the one file that speaks `unsafe` | `wk` |
//!
//! Sign-ins live in one persistent store per manifest (`frame::store_id`), never the default store Safari uses.

pub mod finder;
pub mod frame;
pub mod keys;
pub mod kit;
mod page;
mod render;
mod wk;

pub use page::{after_frame, web_pane, Mode, Source, Status, WebPage, WebPageEvent};
pub use render::{render_html, Appearance};
pub use wk::{in_use, pending, pump};
