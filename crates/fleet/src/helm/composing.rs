//! The line a message to Helm carries ahead of the person's words: where they are in Bridge.
//!
//! **The Sessions' send does the rest.** A tag, a file and a picture reach the Helm Session as they
//! reach any Session's, through `send_known_ahead`, so this is only the line.

use ipc::{HelmContext, HelmScreen};

/// One line naming where the person is, ahead of what they typed — never
/// stored, and never a Job's contents. Helm's note says to read a named Job
/// through its own tools rather than trust this for more than its name.
pub(super) fn where_they_are(context: &HelmContext) -> String {
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
