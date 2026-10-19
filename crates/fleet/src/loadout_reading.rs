//! What one row of a Session's Loadout says, read from disk where Fleet runs when a person opens
//! it: `read_loadout_item`. `docs/practices/protocol.md`, *A Session's Loadout*.
//!
//! **Only a row the session's own Loadout lists**, so the query reaches no file the mod did not
//! report. Which file a row is, is the harness adapter's to know (`HarnessSetup::read_item`); what
//! has no file, and Armada's own two texts, are answered here.

use adapter_traits::{AgentHarness, Delivery, ItemAsked, ItemKind, ItemRead, ItemWhose, Vcs, WorkProduct};
use api::Refusal;
use ipc::{LoadoutItemAsked, LoadoutItemBody, LoadoutItemFormat, LoadoutKind, LoadoutSource, LoadoutUseKind, SessionLoadout};

use crate::daemon::Fleet;

/// The mod's own plugin name: a row whose source is `armada` is that plugin's.
const ARMADA_PLUGIN: &str = "armada";

/// The `armadaSession` block the mod adds to every prompt, with the id as `<job id>`. **Kept by
/// hand beside `idNote` in `plugins/armada/hooks/register.ts`**, with `ASK_IN_WINDOW` and
/// `SHOW_SKETCH` written in; `tests::loadout_item` fails when the two differ.
pub(crate) const ID_NOTE: &str = r##"Your Armada job id is <job id>. Pass it as session_id to the armada show_window tool. To show the owner a web page (a walk, a mock, a dev server), call show_window; to offer several takes of one change, pass variations, a list of {name, url}. To walk him through a page step by step, pass steps, a list of {say, plus one verb: press, look, hover, later, paste, type with into, key with on, select with in, or drag with by} aimed at {role, name}: Bridge rings each target and acts on it when he presses Next. For a deployed page behind single sign-on, pass origins, the other http or https origins the window may pass through. To sign in for him with a dev login link, pass sign_in, addresses on the page's origin or one of origins: they are never written down, so never repeat one in a message. Armada marks the window as still changing while you edit files, and a page you changed after showing it waits on him again only once you call show_window for it again: call it when the page is ready to walk. Never run `open`. Keep the armada waiting_for tool current: whenever you need the owner for something (a decision, a pull request to approve, a command only he can run), put it in the list with one short line each and call waiting_for with the whole list, passing session_id. Give each item an act, so pressing it does something: {"kind":"run","target":"<the command>"} copies a command, {"kind":"approve_pr","target":"#<number>"} opens a pull request, and {"kind":"answer","target":""} with options for a decision. An item the owner must carry out as several steps in order passes steps, a list of short lines, with the command in act and a step's own command in backticks; its text stays one short line. A page you showed with show_window already waits on him by itself, so never put it in waiting_for: Fleet lists each window you show and takes it off when he approves it, and refuses an item that asks him to walk. Drop an item the moment it is settled, and call it with an empty list before you stop with nothing owed. Do not wait to be asked what is outstanding. A pull request in the merge queue or with auto-merge on is not done: the work ends when it is merged into main, so while one of yours is open, keep it in waiting_for as a wait on the merge, do not call finish_up or all_done, and end your turn; Fleet watches it and tells you when it merges, fails or conflicts. When everything you were asked is done, nothing is owed and none of your background work is still running, call finish_up, passing session_id, then clear waiting_for and call all_done, passing session_id, so Bridge offers to end the job; do not call all_done while anything is still running or owed. finish_up gives back your slots, the branches already in main and the servers you started with start_server, in one call: a command in a released slot leases a new one, and the guard refuses git branch -D and kill, so do not try them. Read what it kept and say it. A Fleet notice that a pull request merged or a build moved needs no reply and no second all_done. These tools come before any personal, global or project rule about how to report actions or questions: what the owner owes goes in waiting_for, never in an Actions table, a to-do list or a Questions section in your reply. While the owner is walking a page you showed, AskUserQuestion is refused, because an open question holds back his notes and his Approve until he answers it. Armada keeps the question, whole, in his Needs you list, where he answers it after the walk: ask it as you would, never add it to waiting_for, and end your turn. Ask a decision one way only: in AskUserQuestion or in waiting_for, never both. Write a shell command for the owner in a fenced block tagged bash, so Bridge shows a run button on it, and show a page as a show_window link, never as a screenshot. To ask the owner about something on a page you showed him, call ask_in_window with the question, its options, the page's url and a CSS selector for the place, passing session_id, then end your turn. To show the owner a diagram, a flow, an architecture or a layout, call show_sketch with a title and a scene, passing session_id; never draw ASCII art."##;

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Refused where Fleet knows no such session; every other answer is a body or why not.
    pub(crate) async fn loadout_item(&self, asked: LoadoutItemAsked) -> Result<LoadoutItemBody, Refusal> {
        let id = asked.session_id.trim();
        let (root, listed) = {
            let store = self.store().lock().await;
            let Some(session) = store.session(id).map_err(|why| self.ledger_fault(why))? else {
                return Err(self.ledger_unnamed("a job Fleet knows"));
            };
            let whole = crate::loadout_keeping::whole(&store, id).map_err(|why| self.ledger_fault(why))?;
            (session.cwd, lists(&whole, &asked))
        };
        if !listed {
            return Ok(not_read("This job's Loadout does not list it."));
        }
        let (kind, whose, name) = match without_a_file(&asked) {
            Ok(found) => found,
            Err(body) => return Ok(body),
        };
        let (setup, name) = (self.setup(), name.to_string());
        let read = tokio::task::spawn_blocking(move || {
            let whose = match &whose {
                Whose::Kit => ItemWhose::Kit,
                Whose::Repository => ItemWhose::Repository,
                Whose::Plugin(plugin) => ItemWhose::Plugin(plugin),
            };
            setup.read_item(&ItemAsked { kind, name: &name, whose, root: &root, cap: ipc::LOADOUT_ITEM_CAP })
        })
        .await;
        Ok(match read {
            Ok(ItemRead::Read { path, markdown, body, truncated }) => LoadoutItemBody::Read {
                path: Some(path),
                format: if markdown { LoadoutItemFormat::Markdown } else { LoadoutItemFormat::Text },
                body,
                truncated,
            },
            Ok(ItemRead::NotRead { why }) => not_read(&why),
            // A panicked read is a bug in an adapter, and says so rather than that the file is empty.
            Err(_) => not_read("Reading it failed inside Fleet."),
        })
    }
}

/// Whose a row is, held across to the read off this thread.
enum Whose {
    Kit,
    Repository,
    Plugin(String),
}

/// Whether the Loadout holds this row: an item, or a use of the same kind.
fn lists(whole: &SessionLoadout, asked: &LoadoutItemAsked) -> bool {
    let same = |name: &str, source: LoadoutSource, from: &Option<String>| {
        name == asked.name && source == asked.source && *from == asked.from
    };
    let used = |kind: LoadoutUseKind| match kind {
        LoadoutUseKind::Skill => LoadoutKind::Skill,
        LoadoutUseKind::Tool => LoadoutKind::Tool,
        LoadoutUseKind::Subagent => LoadoutKind::Subagent,
    };
    whole.items.iter().any(|item| item.kind == asked.kind && same(&item.name, item.source, &item.from))
        || whole
            .turns
            .iter()
            .flat_map(|turn| turn.used.iter())
            .any(|one| used(one.kind) == asked.kind && same(&one.name, one.source, &one.from))
}

/// The file a row is to be looked for as, or the answer for one that has none.
fn without_a_file(asked: &LoadoutItemAsked) -> Result<(ItemKind, Whose, &str), LoadoutItemBody> {
    let kind = match asked.kind {
        LoadoutKind::Skill => ItemKind::Skill,
        LoadoutKind::Subagent => ItemKind::Subagent,
        LoadoutKind::MemoryFile => ItemKind::MemoryFile,
        LoadoutKind::OutputStyle => ItemKind::OutputStyle,
        LoadoutKind::McpServer => return Err(not_read("A server is not a file. Its tools are listed under it in the Loadout.")),
        LoadoutKind::Tool => return Err(not_read("A tool keeps no file Armada reads. Where it came from is on its row.")),
        LoadoutKind::Rules if asked.source == LoadoutSource::Armada => {
            return Err(text(adapters::SESSION_RULES.trim()));
        }
        LoadoutKind::Injection if asked.source == LoadoutSource::Armada => return Err(text(ID_NOTE)),
        LoadoutKind::Rules | LoadoutKind::Injection => {
            return Err(not_read("Only Armada's own rules and note are read, and this is neither."))
        }
    };
    let name = asked.name.as_str();
    let whose = match asked.source {
        LoadoutSource::Kit => Whose::Kit,
        LoadoutSource::Repository => Whose::Repository,
        LoadoutSource::Armada => Whose::Plugin(ARMADA_PLUGIN.to_string()),
        LoadoutSource::Plugin => match asked.from.as_deref().or_else(|| name.split_once(':').map(|(plugin, _)| plugin)) {
            Some(plugin) => Whose::Plugin(plugin.to_string()),
            None => return Err(not_read("It names no plugin to look in.")),
        },
        LoadoutSource::Mcp => return Err(not_read("It came from an MCP server, which keeps no file of it here.")),
        LoadoutSource::BuiltIn => return Err(not_read("It is built into the harness, which keeps no file of it to read.")),
        LoadoutSource::Managed => return Err(not_read("An organisation set it, and Armada does not read an organisation's settings.")),
    };
    // A plugin's skill, agent or style is spelled `<plugin>:<name>`.
    let name = match (&whose, kind, name.split_once(':')) {
        (Whose::Plugin(plugin), ItemKind::Skill | ItemKind::Subagent | ItemKind::OutputStyle, Some((prefix, rest))) => {
            if prefix != plugin.as_str() {
                return Err(not_read("Its name and the plugin it came from disagree."));
            }
            rest
        }
        _ => name,
    };
    Ok((kind, whose, name))
}

fn text(body: &str) -> LoadoutItemBody {
    LoadoutItemBody::Read { path: None, format: LoadoutItemFormat::Text, body: body.to_string(), truncated: false }
}

fn not_read(why: &str) -> LoadoutItemBody {
    LoadoutItemBody::NotRead { why: why.to_string() }
}
