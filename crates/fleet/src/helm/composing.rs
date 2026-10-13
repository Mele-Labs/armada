//! A message to Helm as it is taken and as its session is told it: the uploads kept, the tags
//! turned into the line that names them, and the turn assembled from both and the person's words.
//!
//! **The Sessions' pieces, not a second set**: `keep_uploads`, `mention_line`, `addressed` and
//! `UserLine` are what `send_hosted` composes with, so a tag or a file reads to Helm as it reads
//! to a Session.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AskHelm, HelmContext, HelmScreen, SentFile, SessionTag, WireError};

use super::conversation::ConversationKey;
use crate::daemon::Fleet;
use crate::session_host::mention_line;

/// A message with no words, no file and no tag. A 422.
const HELM_MESSAGE_EMPTY: &str = "fleet.helm_message_empty";

/// One message, taken: what the thread keeps of it and what its session is told.
pub(super) struct Message {
    pub text: String,
    pub context: Option<HelmContext>,
    /// Kept under the conversation's uploads, and drawn on the `asked` row.
    pub files: Vec<SentFile>,
    pub tags: Vec<SessionTag>,
    /// The line naming what `tags` point at, written for the session and never drawn.
    pub mentioned: Option<String>,
    /// Where each file that is not a picture is kept.
    pub paths: Vec<String>,
    /// `(media type, base64)` for each picture.
    pub pictures: Vec<(String, String)>,
}

impl Message {
    /// The words of the turn after the brief: where the person is, what they named, what they
    /// said and one line per file. **A words-only message is `where`'s line and the words**, as it
    /// was before anything else could ride with one.
    pub(super) fn told(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(context) = &self.context {
            parts.push(where_they_are(context));
        }
        parts.extend(self.mentioned.clone());
        if !self.text.trim().is_empty() {
            parts.push(self.text.clone());
        }
        parts.extend(self.paths.iter().map(|path| format!("Attached file: {path}")));
        parts.join("\n\n")
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
    /// Take a message: refused where it carries nothing, its uploads kept, its tags addressed.
    /// **Nothing is kept for a message that is refused for another reason**, because this runs
    /// before the conversation counts the message as taken.
    pub(super) async fn helm_message(
        &self,
        key: &ConversationKey,
        asked: AskHelm,
    ) -> Result<Message, Refusal> {
        if asked.text.trim().is_empty() && asked.attachments.is_empty() && asked.mentions.is_empty()
        {
            return Err(Refusal::Unacceptable(WireError::raised(
                HELM_MESSAGE_EMPTY,
                "A message to Helm needs words or something sent with it",
                self.run_id(),
            )));
        }
        let kept = self.keep_uploads(
            &key.uploads_in(&self.host().attachments_dir),
            key.as_str(),
            &asked.attachments,
        )?;
        let addressed = self.addressed(&asked.mentions).await?;
        Ok(Message {
            text: asked.text,
            context: asked.context,
            files: kept.files,
            tags: asked.mentions,
            mentioned: mention_line(&addressed),
            paths: kept.paths,
            pictures: kept.pictures,
        })
    }
}

/// One line naming where the person is, ahead of what they typed — never
/// stored, and never a Job's contents. Helm's brief says to read a named Job
/// through its own tools rather than trust this for more than its name.
fn where_they_are(context: &HelmContext) -> String {
    let mut said = format!("The person is on {}", screen_phrase(context.screen));
    match &context.picked {
        Some(picked) => said.push_str(&format!(", picked on {}", picked.as_str())),
        None => said.push_str(", picked on All repositories"),
    }
    if let Some(chip) = &context.chip {
        said.push_str(&format!(
            ". Job {} is chipped to Helm's message box",
            chip.as_str()
        ));
    }
    if let Some(cursor) = &context.cursor {
        said.push_str(&format!(". The cursor is on Job {}", cursor.as_str()));
    }
    if let Some(studio) = &context.studio {
        said.push_str(&format!(". Studio {} is open", studio.as_str()));
        if let Some(node) = &context.node {
            said.push_str(&format!(", with node {} selected", node.as_str()));
        }
    }
    said.push('.');
    said
}

fn screen_phrase(screen: HelmScreen) -> &'static str {
    match screen {
        HelmScreen::Overview => "Overview",
        HelmScreen::Board => "the Job Board",
        HelmScreen::Manifest => "the Manifest",
        HelmScreen::Cleanup => "Cleanup",
        HelmScreen::Studio => "Studios",
        HelmScreen::Kit => "Kit",
        HelmScreen::Settings => "Settings",
        HelmScreen::Workflows => "Workflows",
        HelmScreen::JobDetail => "a Job's detail",
    }
}
