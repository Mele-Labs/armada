//! Share on a document in the reader, as Classic's `DocumentLayer` shares one: Save as web page and Save as PDF write the
//! page where he picks in the save dialog (`document-save.ts`), and one row per document tool the Job has connected asks
//! the Job to export it there. **The Job does the export with its own MCP tools**; Bridge never calls a server itself.

use std::path::PathBuf;

use gpui::{prelude::*, AnyElement, Context, Role, SharedString, Window};
use ipc::{ArtifactForm, McpServerState, SessionMcp, SessionMcpServer};

use super::sketch_read::SAID_FOR;
use super::JobView;
use crate::dropdown::{Entry, Row};
use crate::labels::labelled;
use crate::look::Look;
use crate::scene::export;

/// A document tool a Job's MCP server may be: its menu row's id, the tool's name, and the server names that mean it, as
/// words of the name in lower case.
pub(super) struct DocumentTool {
    pub id: &'static str,
    pub tool: &'static str,
    names: &'static [&'static str],
}

/// Every document tool Share offers an export to. The first that matches a name wins, so OneDrive is read before Drive.
pub(super) const DOCUMENT_TOOLS: &[DocumentTool] = &[
    DocumentTool { id: "export-notion", tool: "Notion", names: &["notion"] },
    DocumentTool { id: "export-onedrive", tool: "OneDrive", names: &["onedrive", "one drive"] },
    DocumentTool { id: "export-sharepoint", tool: "SharePoint", names: &["sharepoint", "share point"] },
    DocumentTool { id: "export-drive", tool: "Google Drive", names: &["gdrive", "googledrive", "google drive", "drive"] },
    DocumentTool { id: "export-confluence", tool: "Confluence", names: &["confluence", "atlassian"] },
    DocumentTool { id: "export-dropbox", tool: "Dropbox", names: &["dropbox"] },
    DocumentTool { id: "export-box", tool: "Box", names: &["box"] },
    DocumentTool { id: "export-coda", tool: "Coda", names: &["coda"] },
    DocumentTool { id: "export-slite", tool: "Slite", names: &["slite"] },
];

/// The page is laid out this wide for its PDF, in points: A4's width at the 96 a web page counts to the inch.
const PDF_WIDTH: f32 = 794.;

/// The document tool a server is, by the words of its name: `notion`, `team_ai_Notion` and `Notion MCP` are Notion,
/// and `sandbox` is not Box.
pub(super) fn tool_of(server: &str) -> Option<&'static DocumentTool> {
    let lower = server.to_lowercase();
    let words: Vec<&str> = lower.split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty()).collect();
    let spaced = format!(" {} ", words.join(" "));
    DOCUMENT_TOOLS.iter().find(|tool| tool.names.iter().any(|name| spaced.contains(&format!(" {name} "))))
}

/// The tools the Job can export to: one row a tool, from the first connected server that is it. A server not connected
/// gets none.
pub(super) fn export_tools(servers: &[SessionMcpServer]) -> Vec<(&'static DocumentTool, String)> {
    let mut tools: Vec<(&'static DocumentTool, String)> = Vec::new();
    for server in servers.iter().filter(|server| server.state == McpServerState::Connected) {
        if let Some(tool) = tool_of(&server.name).filter(|tool| !tools.iter().any(|(had, _)| had.id == tool.id)) {
            tools.push((tool, server.name.clone()));
        }
    }
    tools
}

/// What an export row sends the Job.
pub(super) fn export_ask(title: &str, artifact: &str, revision: u32, tool: &str, server: &str) -> String {
    format!("Export the document \"{title}\" (artifact {artifact}, revision {revision}) to {tool} with your {server} tools, and reply with the link.")
}

/// The page as one file for somebody with no Armada, as Classic's `savedPage`: the sealed page as the Job wrote it, its
/// sketches already drawn in. Served for a host that draws the kit (`host_kit`), it holds no kit, and its own scripts stay
/// with the policy that allows exactly them by hash and reaches nothing; only the record goes. A page whose policy names a
/// nonce carries the kit inline, so every script is taken out and the nonce turned to none.
pub(super) fn web_page(sealed: &str) -> String {
    const NONCE: &str = "script-src 'nonce-";
    let nonced = sealed.find(NONCE);
    let mut out = without_scripts(sealed, |tag| nonced.is_some() || tag.contains("id=\"armada-page-record\""));
    if let Some(from) = nonced.and_then(|_| out.find(NONCE)) {
        if let Some(end) = out[from + NONCE.len()..].find('\'') {
            out.replace_range(from..from + NONCE.len() + end + 1, "script-src 'none'");
        }
    }
    out
}

/// `html` with each script whose opening tag, lowercased, `drop` answers true for taken out whole.
fn without_scripts(html: &str, drop: impl Fn(&str) -> bool) -> String {
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    let mut at = 0;
    while let Some(found) = lower[at..].find("<script") {
        let start = at + found;
        let after = start + "<script".len();
        // `<scripted>` is not a script.
        if !lower.as_bytes().get(after).is_some_and(|byte| byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/')) {
            out.push_str(&html[at..after]);
            at = after;
            continue;
        }
        let end = lower[after..].find("</script").map_or(html.len(), |close| {
            let close = after + close;
            lower[close..].find('>').map_or(html.len(), |end| close + end + 1)
        });
        let tag = &lower[start..lower[start..].find('>').map_or(lower.len(), |close| start + close + 1)];
        out.push_str(&html[at..start]);
        if !drop(tag) {
            out.push_str(&html[start..end]);
        }
        at = end;
    }
    out.push_str(&html[at..]);
    out
}

/// The document Share was opened on, and the revision he reads.
pub(super) struct Shared {
    pub job: SharedString,
    pub reader: String,
    pub title: String,
    pub artifact: String,
    pub revision: u32,
}

impl JobView {
    /// Share, its glyph the one the sketch's rail shares with. S does the same where the reader holds the keys.
    pub(super) fn reader_share_button(&self, id: &SharedString, name: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (job, name_c) = (id.clone(), name.to_string());
        labelled(
            super::head::icon_button(look, "reader-share", "external-link", "Share, S".into(), false, false)
                .id(SharedString::from(format!("reader-share:{name}")))
                .on_click(cx.listener(move |this, _, window, cx| this.reader_share(&job, &name_c, window, cx))),
            Role::Button,
            "Share",
        )
        .into_any_element()
    }

    /// Share's menu, with the Job's servers read as it opens, so a tool connected since is offered.
    pub(super) fn reader_share(&mut self, id: &SharedString, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_of(id, name) else { return };
        let Some(subject) = self.subject_of(id, held.kind, &held.key, cx).filter(|subject| subject.form == ArtifactForm::Page) else { return };
        let shared = Shared { job: id.clone(), reader: name.to_string(), title: subject.title, artifact: subject.artifact, revision: held.viewing.unwrap_or(subject.newest) };
        if self.fleet.is_none() {
            self.open_share(shared, &crate::mock::session_mcp().servers, window, cx);
            return;
        }
        let path = format!("/sessions/mcp?session_id={}", super::wire::encoded(id));
        let done: super::wire::Done = Box::new(move |this, answer, window, cx| {
            // Servers Fleet could not read leave the menu its saves.
            let servers = answer.ok().and_then(|bytes| ipc::decode::<SessionMcp>("SessionMcp", &bytes).ok()).map(|mcp| mcp.servers).unwrap_or_default();
            this.open_share(shared, &servers, window, cx);
        });
        self.get(path, window, cx, done);
    }

    /// The menu under Share: the two saves and Add to the repo, as Classic's, then under a rule one export a document tool
/// the Job has connected.
    pub(super) fn open_share(&mut self, shared: Shared, servers: &[SessionMcpServer], window: &mut Window, cx: &mut Context<Self>) {
        let tools = export_tools(servers);
        let off = if self.closed(&shared.job, cx) { Some(SharedString::from("The Job has ended")) } else { self.off_to(&shared.job, super::composer::MESSAGE, cx) };
        let asks = |row: Row| {
            Entry::Row(match &off {
                Some(why) => row.off(why.clone()),
                None => row,
            })
        };
        let mut entries = vec![Entry::Row(Row::new("html", "Save as web page")), Entry::Row(Row::new("pdf", "Save as PDF")), asks(Row::new("repo", "Add to the repo"))];
        if !tools.is_empty() {
            entries.push(Entry::Rule);
            entries.extend(tools.iter().map(|(tool, _)| asks(Row::new(tool.id, format!("Export to {}", tool.tool)).keeps_open())));
        }
        let book = cx.try_global::<crate::labels::LabelBook>();
        let button = book.and_then(|book| book.find(window.window_handle().window_id(), Role::Button, "Share")).map(|one| one.bounds);
        let at = button.map_or_else(|| window.mouse_position(), crate::dropdown::under);
        let back = Some(self.row_focus.clone());
        self.open_button_menu(
            "Share",
            entries,
            at,
            back,
            move |this, picked, window, cx| match picked {
                "html" | "pdf" => this.reader_save(&shared, picked == "pdf", cx),
                // As Classic's Add to the repo: the page is ignored by git, so the commit forces it in. Its id is Fleet's
                // copy, outside every checkout, so the Job is named the page as it writes it in its own.
                "repo" if this.shown.as_ref() == Some(&shared.job) => {
                    let page = shared.artifact.rfind(".armada/documents/").map_or(shared.artifact.as_str(), |at| &shared.artifact[at..]);
                    this.send_line(format!("Commit {page} to the repo. It is gitignored, so add it with git add -f."), window, cx)
                }
                _ => {
                    if let Some((tool, server)) = tools.iter().find(|(tool, _)| tool.id == picked) {
                        this.reader_export(&shared, tool, server, window, cx);
                    }
                }
            },
            window,
            cx,
        );
    }

    /// The Job is asked to export it, as the composer sends a line, and the row says it went.
    fn reader_export(&mut self, shared: &Shared, tool: &DocumentTool, server: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.shown.as_ref() != Some(&shared.job) {
            return;
        }
        self.send_line(export_ask(&shared.title, &shared.artifact, shared.revision, tool.tool, server), window, cx);
        let Some(menu) = self.pop.as_ref().map(|pop| pop.menu.clone()) else { return };
        menu.update(cx, |menu, cx| {
            for entry in &mut menu.entries {
                if let Entry::Row(row) = entry {
                    if row.id == tool.id {
                        row.label = "Sent to the Job".into();
                        row.off = Some(format!("Export to {} was sent", tool.tool).into());
                    }
                }
            }
            cx.notify();
        });
    }

    /// Save as web page or PDF: the save dialog, then the file. In a walk no dialog opens and nothing is written, but the
    /// file is made, so a PDF WebKit cannot make is said there too.
    fn reader_save(&mut self, shared: &Shared, pdf: bool, cx: &mut Context<Self>) {
        let name = export::file_name(&shared.title, if pdf { "pdf" } else { "html" });
        let Some(page) = self.reader_page(&shared.job, &shared.reader).and_then(|tab| tab.read(cx).html().map(web_page)) else {
            crate::toasts::tell(format!("{name} was not saved: the page has not arrived from Fleet."), cx);
            return;
        };
        let picked = self.fleet.is_some().then(|| {
            let folder = std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Downloads")).unwrap_or_else(std::env::temp_dir);
            cx.prompt_for_new_path(&folder, Some(&name))
        });
        let (job, reader) = (shared.job.clone(), shared.reader.clone());
        cx.spawn(async move |this, cx| {
            // A dialog he closes says nothing, as the sketch's save.
            let path = match picked {
                Some(picked) => {
                    let Ok(Ok(Some(path))) = picked.await else { return };
                    Some(path)
                }
                None => None,
            };
            let saved = path.as_ref().and_then(|path| path.file_name()).map_or_else(|| name.clone(), |file| file.to_string_lossy().into_owned());
            let bytes = if pdf { cx.update(|cx| crate::web::pdf_of(page, PDF_WIDTH, cx)).await } else { Ok(page.into_bytes()) };
            let written = match (bytes, path) {
                (Ok(bytes), Some(path)) => cx.background_executor().spawn(async move { std::fs::write(&path, bytes).map_err(|why| why.to_string()) }).await,
                (Ok(_), None) => Ok(()),
                (Err(why), _) => Err(why),
            };
            let _ = this.update(cx, |this, cx| match written {
                Ok(()) => this.reader_said_shared(&job, &reader, format!("Saved {saved}.").into(), cx),
                Err(why) => crate::toasts::tell(format!("{saved} was not saved: {why}."), cx),
            });
        })
        .detach();
    }

    /// A save is otherwise silent, so the band says it for a moment, as the sketch's band does.
    fn reader_said_shared(&mut self, id: &SharedString, name: &str, said: SharedString, cx: &mut Context<Self>) {
        let Some(held) = self.reader_mut(id, name) else { return };
        held.shared = Some(said.clone());
        cx.notify();
        let (job, reader) = (id.clone(), name.to_string());
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAID_FOR).await;
            let _ = this.update(cx, |this, cx| {
                if let Some(held) = this.reader_mut(&job, &reader).filter(|held| held.shared.as_ref() == Some(&said)) {
                    held.shared = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use gpui::{px, size};

    use super::super::parts_fold::{redraw, rooted_job};
    use super::*;

    fn server(name: &str, state: McpServerState) -> SessionMcpServer {
        SessionMcpServer { name: name.into(), state, error: None, scope: None, tools: 0 }
    }

    #[test]
    fn a_server_name_is_read_as_the_document_tool_it_names() {
        let named = |server: &str| tool_of(server).map(|tool| tool.tool);
        assert_eq!(named("notion"), Some("Notion"));
        assert_eq!(named("team_ai_Notion"), Some("Notion"), "case and the words round it do not matter");
        assert_eq!(named("plugin:productivity:notion"), Some("Notion"));
        for drive in ["drive", "gdrive", "google-drive", "Google Drive", "googledrive"] {
            assert_eq!(named(drive), Some("Google Drive"), "{drive}");
        }
        assert_eq!(named("confluence"), Some("Confluence"));
        assert_eq!(named("atlassian"), Some("Confluence"));
        assert_eq!(named("onedrive"), Some("OneDrive"), "OneDrive is not Google Drive");
        assert_eq!(named("one-drive"), Some("OneDrive"));
        assert_eq!(named("SharePoint"), Some("SharePoint"));
        assert_eq!((named("Dropbox"), named("box"), named("coda"), named("slite")), (Some("Dropbox"), Some("Box"), Some("Coda"), Some("Slite")));
        assert_eq!(named("sandbox"), None, "a word that holds a name is not it");
        assert_eq!(named("armada-fleet"), None);
        assert_eq!(named("codacy"), None);
    }

    #[test]
    fn the_export_ask_names_the_document_its_revision_the_tool_and_the_server() {
        assert_eq!(
            export_ask("Stale branch design", "design", 2, "Notion", "notion"),
            "Export the document \"Stale branch design\" (artifact design, revision 2) to Notion with your notion tools, and reply with the link."
        );
    }

    #[test]
    fn the_web_page_keeps_the_page_and_carries_no_script() {
        let sealed = r#"<html><head><meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'nonce-abc=='; connect-src 'none'"></head><body><scripted>kept</scripted><p>Words.</p><script type="application/json" nonce="abc==">{"a":1}</script><SCRIPT nonce="abc==">run()</SCRIPT></body></html>"#;
        let page = web_page(sealed);
        let lower = page.to_ascii_lowercase();
        assert!(!lower.contains("<script ") && !lower.contains("<script>"), "{page}");
        assert!(page.contains("script-src 'none'") && !page.contains("nonce"), "{page}");
        assert!(page.contains("<scripted>kept</scripted><p>Words.</p></body>"), "{page}");
    }

    #[test]
    fn the_web_page_of_a_hosted_page_keeps_its_own_scripts_and_their_policy_and_drops_the_record() {
        let policy = "default-src 'none'; style-src 'unsafe-inline'; img-src data:; script-src 'sha256-AAAA'; connect-src 'none'; form-action 'none'; base-uri 'none'";
        let sealed = format!(r#"<html><head><meta http-equiv="Content-Security-Policy" content="{policy}"></head><body><p>Words.</p><script>flip()</script><script type="application/json" id="armada-page-record">{{"presses":[]}}</script></body></html>"#);
        let page = web_page(&sealed);
        assert!(page.contains(&format!("content=\"{policy}\"")), "the policy that allows exactly the page's scripts stays: {page}");
        assert!(page.contains("<p>Words.</p><script>flip()</script></body>"), "{page}");
        assert!(!page.contains("armada-page-record") && !page.contains("presses"), "{page}");
    }

    #[gpui::test]
    fn share_lists_export_to_notion_for_a_connected_server_and_nothing_for_one_that_is_off(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "reader", "draft-job", size(px(1400.), px(900.)), |_| {});
        let servers = [server("notion", McpServerState::Connected), server("confluence", McpServerState::Off), server("armada-fleet", McpServerState::Connected)];
        view.update_in(cx, |view, window, cx| {
            let shared = Shared { job: "draft-job".into(), reader: "page:design".into(), title: "Stale branch design".into(), artifact: "design".into(), revision: 2 };
            view.open_share(shared, &servers, window, cx);
        });
        redraw(cx);
        let rows: Vec<String> = view.read_with(cx, |view, cx| {
            let menu = view.pop.as_ref().expect("the Share menu is open").menu.read(cx);
            menu.entries.iter().filter_map(|entry| match entry {
                Entry::Row(row) => Some(row.label.to_string()),
                _ => None,
            })
            .collect()
        });
        assert_eq!(rows, ["Save as web page", "Save as PDF", "Add to the repo", "Export to Notion"], "Confluence is off and Fleet's own door is no document tool");
    }
}
