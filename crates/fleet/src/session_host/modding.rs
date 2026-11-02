//! `start_mod_session`: a Mod Job, a hosted Job on Armada's own repository for modding Armada
//! itself, started on what the person pointed at. Where that repository is not served, the Job runs
//! in the mods folder and makes mods only. `docs/concepts/mods.md`, *A Mod Job*.
//!
//! **The first message is a pinned note's**, sent by `send_session_note`'s road with the Mod brief
//! ahead of his words, so the turn runs under the rules any message is under. Where he pointed at
//! nothing it is his words alone, sent by `send_session_message`'s road with the brief ahead.
//! **Where he gave neither** the Job starts empty, and the brief rides ahead of his first message
//! when he writes it, which `mod_message_sent` adds.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    ManifestId, ModAim, ModSessionStarted, SendSessionMessage, SendSessionNote, SessionRecord, SessionRow, SessionVoice,
    StartModSession, StartSession,
};

use super::serving::title_of;
use super::Marked;
use crate::daemon::Fleet;

/// A start that names both a note and his words, or words with nothing in them. A 422.
const UNACCEPTABLE_MOD_START: &str = "fleet.unacceptable_mod_start";

/// A Mod Job's title where neither the start nor his words give one.
const UNTITLED: &str = "Mod";

/// What a Mod Job is told ahead of the note. **Sent as it stands**, so it is short and plain.
const BRIEF: &str = "This is a Mod Job: the owner pressed Mod on a spot in Armada, and the change is to Armada itself. \
Load the `armada:armada-mods` skill and reach for a mod first: a theme, a layout, words, a Fleet rule, a tracker or a tab. \
A mod applies live once `validate_mod` passes, and the owner switches it off from this Job if he does not like it. \
Where the ask needs more than a mod can say, tell the owner so, change Armada's source on this Job's branch, \
then put `scripts/preview --restart` in waiting_for as a run item, so he can bring it in. \
When he asks for a panel, a view or anything drawn, build a tab mod: it opens in this Job's panes as you build it, \
so draw its frame first and add to it in small checked steps. Talk to him about the panel, \
never about mods, branches, pull requests or landing.";
/// [`BRIEF`] for a Mod Job in the mods folder, where this Fleet does not serve Armada's own repository.
const MODS_FOLDER_BRIEF: &str = "This is a Mod Job in the mods folder: the owner pressed Mod on a spot in Armada, \
and this Fleet does not serve Armada's own repository, so the change is a mod. \
Load the `armada:armada-mods` skill and make a theme, a layout, words, a Fleet rule, a tracker or a tab. \
A mod applies live once `validate_mod` passes, and the owner switches it off from this Job if he does not like it. \
Armada's source is not here to change: where the ask needs more than a mod can say, tell the owner so. \
When he asks for a panel, a view or anything drawn, build a tab mod: it opens in this Job's panes as you build it, \
so draw its frame first and add to it in small checked steps. Talk to him about the panel, \
never about mods, branches, pull requests or landing.";

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
    /// Start a Mod Job and send it the brief and the note.
    pub(crate) async fn mod_job_started(self: Arc<Self>, start: StartModSession) -> Result<ModSessionStarted, Refusal> {
        let StartModSession { title, model, effort, aim, note, said, from_job, from_tab } = start;
        let said = said.map(|words| words.trim().to_string());
        let first = match (note, said) {
            (Some(note), None) => First::Note(note),
            (None, Some(words)) if !words.is_empty() => First::Words(words),
            (None, Some(_)) => {
                return Err(self.hosted_refusal(UNACCEPTABLE_MOD_START, "a Mod Job needs his words: `said` is empty"))
            }
            (None, None) => First::Nothing,
            (Some(_), Some(_)) => {
                return Err(self.hosted_refusal(UNACCEPTABLE_MOD_START, "a Mod Job takes a `note` or `said`, not both"))
            }
        };
        let (manifest, marked, brief) = match self.armadas_own() {
            Some(manifest) => (manifest, Marked::Mod, BRIEF),
            None => {
                let mods = self.mods_dir();
                std::fs::create_dir_all(&mods)
                    .map_err(|why| self.mod_fault(format!("the mods folder could not be made: {why}")))?;
                // Not read: a Job in the mods folder names no Manifest.
                (ManifestId::carried(""), Marked::ModsFolder, MODS_FOLDER_BRIEF)
            }
        };
        let brief = match aim {
            Some(aim) => format!("{brief}\n\n{}", aim_line(aim, marked == Marked::ModsFolder)),
            None => brief.to_string(),
        };
        let title = title
            .map(|title| title.trim().to_string())
            .filter(|title| !title.is_empty())
            .or_else(|| title_of(first.words()))
            .unwrap_or_else(|| UNTITLED.to_string());
        let begun = StartSession {
            manifest_id: manifest,
            title: Some(title.clone()),
            model,
            effort,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held: false,
            project_rows: Vec::new(),
        };
        let session_id = self.started_hosted(begun, marked).await?.id;
        // The Custom tab it builds, or that it builds an instrument: kept on the hosting record, and published again so Bridge draws no row for it.
        let from_job = from_job.filter(|job| !job.is_empty());
        let from_tab = from_tab.filter(|tab| !tab.is_empty());
        let mod_of_bay = aim == Some(ModAim::Instrument);
        if from_job.is_some() || from_tab.is_some() || mod_of_bay {
            {
                let mut store = self.store().lock().await;
                if let Some(mut hosting) = store.hosting(session_id.as_str()).map_err(|why| self.ledger_fault(why))? {
                    hosting.from_job = from_job;
                    hosting.from_tab = from_tab;
                    hosting.mod_of_bay = mod_of_bay;
                    store.keep_hosting(&hosting).map_err(|why| self.ledger_fault(why))?;
                }
            }
            self.published_hosted(session_id.as_str()).await?;
        }
        // **Not an error**: the Job exists, so the answer is 200 and `send_failed` says why
        // nothing was sent.
        let sent = match first {
            First::Nothing => Ok(()),
            First::Note(note) => Arc::clone(&self)
                .note_sent_after(SendSessionNote { session_id: session_id.clone(), note }, Some(&brief))
                .await
                .map(|_| ()),
            First::Words(words) => Arc::clone(&self).words_sent_after(session_id.clone(), words, &brief).await,
        };
        let send_failed = sent.err().map(|refused| refused.error().message.clone());
        Ok(ModSessionStarted { session_id, title, send_failed })
    }
}

/// The width a Mod Job's title takes of his first words, as Bridge's `startModJobFrom` cuts it.
const TITLE_WIDTH: usize = 40;

/// What a Mod Job's first message is made of.
enum First {
    /// What he pointed at.
    Note(ipc::SessionNote),
    /// His words, trimmed and not empty.
    Words(String),
    /// Neither: an empty Mod Job, sent nothing until he writes in it.
    Nothing,
}

impl First {
    fn words(&self) -> &str {
        match self {
            First::Note(note) => &note.said,
            First::Words(words) => words,
            First::Nothing => "",
        }
    }
}

/// The line that names what he picked, between the brief and his words. `source` in the mods
/// folder says it cannot be made there.
fn aim_line(aim: ModAim, in_mods_folder: bool) -> &'static str {
    match aim {
        ModAim::Theme => "He is aiming for: a theme (colours).",
        ModAim::Layout => "He is aiming for: a layout (which tabs and panels are drawn, and in what order).",
        ModAim::Words => "He is aiming for: words (a pattern Bridge links, and where it opens).",
        ModAim::Fleet => "He is aiming for: a Fleet rule (what Fleet does with a notice it would send).",
        ModAim::Tracker => "He is aiming for: a tracker (an issue tracker Armada has no adapter for).",
        ModAim::Commands => "He is aiming for: commands (rows in the command palette that start a Job, open an address, run a line or chain other rows).",
        ModAim::Tab => "He is aiming for: a tab (a mod of kind `tab`, a React component Bridge loads into a pane, with the whole Fleet API). Scaffold it with `kind` `tab` and your own `session_id`, and edit the same mod when he asks for a change. He watches the tab while you build it: first write a version that draws its frame and run `validate_mod`, then add one part at a time and run `validate_mod` after each. Never write the whole tab before the first check.",
        ModAim::Instrument => "He is aiming for: an instrument (a mod of kind `instrument`, one file `instrument.js`, an ES module whose default export is a React component with the prop `host`, seated in a bay of the Cockpit console). Build this and not a tab.\n\
Steps. 1: call `scaffold_mod` with `kind` `instrument` and your own `session_id`; the starter passes as it is. 2: edit `instrument.js` in the `path` it answers, and no other file. 3: run `validate_mod` after each edit and fix every problem it names. 4: edit the same mod when he asks for a change; the bay reloads when the file changes.\n\
Contract. Bridge keeps the component mounted while its panel is closed. `host.face(paint)` registers what is drawn on the bay, and what the component returns stands in the panel. `paint` is `(draw, { w, h, t, ink }) => void` on a 2D canvas context, and `ink` holds the colours `run`, `ask`, `bad`, `good`, `fg`, `muted`, `subtle` and `off`. `host.face(paint, { moves: true })` asks for a repaint once a second; otherwise a face repaints only when the component calls `host.face` again. There is no `import`: everything the instrument uses comes through `host`.",
        ModAim::Source if in_mods_folder => {
            "He is aiming for: a change to Armada's source, which cannot be made here. Tell him so if a mod cannot do it."
        }
        ModAim::Source => "He is aiming for: a change to Armada's source.",
    }
}

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
    /// `send_session_message`: a message to a Job. **A Mod Job that has none of his yet is sent the
    /// brief ahead of it**, as a start with words is, and an untitled one is retitled `Mod` and the
    /// first line of what he wrote. **His `aim` rides with that first message only**, a line between
    /// the brief and his words; on any other message it is ignored. The thread keeps his words alone.
    pub(crate) async fn mod_message_sent(
        self: Arc<Self>,
        terminal: Option<store::KeptSession>,
        sent: SendSessionMessage,
        aim: Option<ModAim>,
    ) -> Result<SessionRecord, Refusal> {
        let words = sent.text.trim().to_string();
        let id = sent.session_id.as_str().to_string();
        let owed = match terminal.is_none() && !words.is_empty() {
            true => self.brief_owed(&id, aim).await?,
            false => None,
        };
        let Some((brief, untitled)) = owed else { return self.send_known(terminal, sent).await };
        if untitled {
            let title = format!("{UNTITLED} {}", cut_at_a_word(&words));
            let mut store = self.store().lock().await;
            if let Some(mut row) = store.session(&id).map_err(|why| self.ledger_fault(why))? {
                row.title = Some(title);
                store.keep_session(&row).map_err(|why| self.ledger_fault(why))?;
            }
        }
        let instead = format!("{brief}\n\n{words}");
        self.send_known_added(None, sent, None, Some(instead)).await
    }

    /// The brief a Mod Job is owed ahead of his first message, with the line naming his `aim` after
    /// it, and whether it still has the title it was started with. None for any other Job, and for a
    /// Mod Job he has already written in.
    async fn brief_owed(&self, id: &str, aim: Option<ModAim>) -> Result<Option<(String, bool)>, Refusal> {
        let (session, hosting) = self.session_and_hosting(id).await?;
        if !hosting.mod_job {
            return Ok(None);
        }
        let spoken = self
            .rows_of(id)
            .await?
            .iter()
            .any(|row| matches!(row, SessionRow::Message { from: SessionVoice::You, .. }));
        let in_mods_folder = hosting.manifest_id.is_empty();
        let mut brief = (if in_mods_folder { MODS_FOLDER_BRIEF } else { BRIEF }).to_string();
        if let Some(aim) = aim {
            brief = format!("{brief}\n\n{}", aim_line(aim, in_mods_folder));
        }
        Ok((!spoken).then_some((brief, session.title.as_deref() == Some(UNTITLED))))
    }

    /// His words as a message, with `brief` ahead of them in what the agent reads and the thread
    /// keeping the words alone: `note_sent_after` for a Mod Job pressed on nothing.
    async fn words_sent_after(
        self: Arc<Self>,
        session_id: ipc::SessionId,
        words: String,
        brief: &str,
    ) -> Result<(), Refusal> {
        let terminal = self.terminal_session(session_id.as_str()).await?;
        let instead = format!("{brief}\n\n{words}");
        let sent = SendSessionMessage { session_id, text: words, attachments: Vec::new(), mentions: Vec::new() };
        self.send_known_added(terminal, sent, None, Some(instead)).await.map(|_| ())
    }
}

/// The first line of `words`, cut at a word where it runs past [`TITLE_WIDTH`].
fn cut_at_a_word(words: &str) -> String {
    let line = words.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default();
    if line.chars().count() <= TITLE_WIDTH {
        return line.to_string();
    }
    let cut: String = line.chars().take(TITLE_WIDTH).collect();
    match cut.rfind(' ') {
        Some(at) => cut[..at].trim_end().to_string(),
        None => cut,
    }
}
