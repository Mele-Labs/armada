//! Revert to the build before, asked from the app menu. Mirrors
//! `apps/desktop/src/main/revert-asked.ts`, and says every string as it does: a native confirm,
//! then Fleet's own revert (`POST /fleet/build/revert`, no body), and where Fleet is not connected
//! to ask, the undo the build before carries (`builds/previous/armada build revert`), run detached.
//!
//! What is pure is here and tested: whether the item is on, the confirm's lines and buttons, the
//! sentence a refusal becomes and where the undo is. [`ask`] is the flow, which needs a window.

use std::path::PathBuf;

use gpui::{AsyncApp, Entity, PromptButton, PromptLevel};
use ipc::{FleetBuildReport, KeptBuildPrevious, WireError};

use crate::editor;
use crate::fleet::FleetConnection;
use crate::log::Note;
use crate::runtime_file;
use crate::transport;

/// The app menu's item, as `REVERT_ITEM` spells it.
pub const REVERT_ITEM: &str = "Revert to the build before…";

pub const NO_BUILD_BEFORE: &str = "No build before this one is kept";

const ROUTE: &str = "/fleet/build/revert";

/// True while a revert can be asked: a build before is kept and nothing is changing the build.
pub fn can_revert(report: Option<&FleetBuildReport>) -> bool {
    report.is_some_and(|report| {
        report.previous.is_some() && !report.reverting && report.restarting.is_none() && !report.swapping
    })
}

/// What a dialog is asked for: Electron's `RevertBox`, less the icon, which is always the information one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sheet {
    pub message: String,
    pub detail: String,
    pub buttons: Vec<&'static str>,
    pub default_id: usize,
    pub cancel_id: usize,
}

impl Sheet {
    /// A message and an OK.
    pub fn info(message: &str, detail: &str) -> Sheet {
        Sheet { message: message.into(), detail: detail.into(), buttons: vec!["OK"], default_id: 0, cancel_id: 0 }
    }

    /// The order the buttons go to the platform in. **Its first button answers Return**, and
    /// the `defaultId` of Electron's box is what answers it here, so that button goes first.
    fn order(&self) -> Vec<usize> {
        let rest = (0..self.buttons.len()).filter(|at| *at != self.default_id);
        std::iter::once(self.default_id).chain(rest).collect()
    }

    pub fn answers(&self) -> Vec<PromptButton> {
        self.order().into_iter().map(|at| PromptButton::new(self.buttons[at])).collect()
    }

    /// The index in `buttons` of the answer the platform reports by its place in [`Sheet::answers`].
    pub fn pressed(&self, answered: usize) -> usize {
        self.order().get(answered).copied().unwrap_or(self.cancel_id)
    }
}

/// The confirm: what the build before is, and what going back to it changes.
pub fn confirm_of(previous: &KeptBuildPrevious) -> Sheet {
    let mut lines = vec![format!(
        "Built {}, commit {}",
        previous.built_at.as_str(),
        previous.commit.chars().take(8).collect::<String>()
    )];
    if !previous.takes_out.is_empty() {
        lines.push(format!("Takes out: {}", previous.takes_out.join(", ")));
    }
    if previous.bridge_reopens {
        lines.push("Bridge reopens".into());
    }
    Sheet {
        message: "Revert to the build before?".into(),
        detail: lines.join("\n"),
        buttons: vec!["Revert", "Cancel"],
        default_id: 1,
        cancel_id: 1,
    }
}

/// How Fleet answered the revert.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Started,
    /// Fleet's own sentence.
    Refused(String),
    /// Fleet did not answer, or answered with no sentence: why, as the transport said it.
    Transport(String),
}

pub fn outcome_of(read: Result<(u16, Vec<u8>), String>) -> Outcome {
    match read {
        Ok((status, _)) if (200..300).contains(&status) => Outcome::Started,
        Ok((status, bytes)) => match ipc::decode::<WireError>("WireError", &bytes) {
            Ok(error) => Outcome::Refused(error.message),
            Err(_) => Outcome::Transport(format!("Fleet answered {status}")),
        },
        Err(why) => Outcome::Transport(why),
    }
}

/// The reason line of a refusal or a Fleet that did not answer, as the Fleet panel words it.
pub fn why_not(outcome: &Outcome) -> &str {
    match outcome {
        Outcome::Started => "",
        Outcome::Refused(message) | Outcome::Transport(message) => message,
    }
}

/// The build before's own `armada`, found the way the app finds `fleet.json`: beside it, under `builds/previous`.
pub fn undo_path(home: Option<&str>) -> Option<PathBuf> {
    Some(runtime_file::path(home)?.parent()?.join("builds").join("previous").join("armada"))
}

/// Present `sheet` on the window in front, and answer the index in `sheet.buttons` of the button
/// pressed. `None` where no window is open to hold it.
async fn show(cx: &mut AsyncApp, sheet: &Sheet) -> Option<usize> {
    let answers = sheet.answers();
    let detail = (!sheet.detail.is_empty()).then_some(sheet.detail.as_str());
    let receiver = cx.update(|cx| {
        let window = cx.active_window().or_else(|| cx.windows().first().copied())?;
        window.update(cx, |_, window, cx| window.prompt(PromptLevel::Info, &sheet.message, detail, &answers, cx)).ok()
    })?;
    receiver.await.ok().map(|answered| sheet.pressed(answered))
}

/// Show a message with an OK, and wait for it.
pub async fn tell(cx: &mut AsyncApp, message: &str, detail: &str) {
    show(cx, &Sheet::info(message, detail)).await;
}

/// The whole of it. **All of it runs here and not in a screen**, so it works with any screen showing.
pub async fn ask(fleet: Entity<FleetConnection>, cx: &mut AsyncApp) {
    let (report, port) = cx.update(|cx| {
        let held = fleet.read(cx).held();
        (held.build.clone(), held.port())
    });
    let Some(previous) = report.and_then(|report| report.previous) else {
        tell(cx, NO_BUILD_BEFORE, "").await;
        return;
    };
    if show(cx, &confirm_of(&previous)).await != Some(0) {
        return;
    }
    let Some(port) = port else {
        // Fleet is not there to ask, so the build before's own undo is run.
        let home = std::env::var("HOME").ok();
        match undo_path(home.as_deref()).filter(|file| file.is_file()) {
            Some(file) => {
                if let Err(why) = editor::start_detached(&file, &["build".to_string(), "revert".to_string()]) {
                    Note::warn("the build before's undo did not start").field("why", why).emit();
                }
            }
            None => tell(cx, "Fleet is not connected", "The build before has no undo to run").await,
        }
        return;
    };
    // A runtime of its own on a background thread: the main thread waits on nothing.
    let read = cx
        .background_executor()
        .spawn(async move {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
            runtime.block_on(transport::post_bare(port, ROUTE))
        })
        .await;
    let outcome = outcome_of(read);
    if outcome == Outcome::Started {
        Note::info("a revert to the build before was started").emit();
        let _ = fleet.update(cx, |fleet, cx| fleet.mark_reverting(cx));
    } else {
        tell(cx, "Revert did not start", why_not(&outcome)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(wire: &str) -> FleetBuildReport {
        ipc::decode("build", wire.as_bytes()).expect("a build report")
    }

    fn kept(extra: &str) -> String {
        format!(r#"{{"on":"main","previous":{{"built_at":"2026-11-05T16:20:00Z","commit":"{}"{extra}}}}}"#, "b".repeat(40))
    }

    #[test]
    fn the_item_is_on_while_a_build_before_is_kept_and_nothing_is_changing_the_build() {
        assert!(can_revert(Some(&report(&kept("")))));
        assert!(!can_revert(None));
        assert!(!can_revert(Some(&report(r#"{"on":"main"}"#))), "no build before");
    }

    #[test]
    fn the_item_is_off_while_the_build_is_changing() {
        let previous = format!(r#""previous":{{"built_at":"2026-11-05T16:20:00Z","commit":"{}"}}"#, "b".repeat(40));
        for changing in [r#""reverting":true"#, r#""restarting":"main""#, r#""swapping":true"#] {
            let wire = format!(r#"{{"on":"main",{previous},{changing}}}"#);
            assert!(!can_revert(Some(&report(&wire))), "{changing}");
        }
    }

    #[test]
    fn the_confirm_says_when_and_which_commit_and_nothing_else_where_nothing_changes() {
        let previous = report(&kept("")).previous.unwrap();
        let sheet = confirm_of(&previous);
        assert_eq!(sheet.message, "Revert to the build before?");
        assert_eq!(sheet.detail, "Built 2026-11-05T16:20:00Z, commit bbbbbbbb");
        assert_eq!(sheet.buttons, ["Revert", "Cancel"]);
        assert_eq!((sheet.default_id, sheet.cancel_id), (1, 1));
    }

    #[test]
    fn the_confirm_names_what_it_takes_out_and_that_bridge_reopens() {
        let previous = report(&kept(r#","takes_out":["Fix the list","armada/b"],"bridge_reopens":true"#)).previous.unwrap();
        assert_eq!(
            confirm_of(&previous).detail,
            "Built 2026-11-05T16:20:00Z, commit bbbbbbbb\nTakes out: Fix the list, armada/b\nBridge reopens"
        );
    }

    #[test]
    fn cancel_is_the_button_return_answers_and_a_press_maps_back_to_its_place_in_the_sheet() {
        let sheet = confirm_of(&report(&kept("")).previous.unwrap());
        let labels: Vec<String> = sheet.answers().iter().map(|button| button.label().to_string()).collect();
        assert_eq!(labels, ["Cancel", "Revert"]);
        assert_eq!(sheet.pressed(0), 1, "Cancel");
        assert_eq!(sheet.pressed(1), 0, "Revert");
        assert_eq!(sheet.pressed(7), 1, "an answer nobody offered is a Cancel");
    }

    #[test]
    fn an_info_sheet_is_one_ok() {
        let sheet = Sheet::info(NO_BUILD_BEFORE, "");
        assert_eq!((sheet.message.as_str(), sheet.buttons.as_slice()), ("No build before this one is kept", ["OK"].as_slice()));
        assert_eq!(sheet.pressed(0), 0);
    }

    #[test]
    fn a_refusal_shows_fleets_message_and_an_unreadable_answer_says_its_status() {
        let error = WireError::raised("fleet.not_revertible", "No build before this one is kept", ipc::RunId::carried("run"));
        let refusal = ipc::encode(&error).expect("it encodes").into_bytes();
        assert_eq!(outcome_of(Ok((409, refusal))), Outcome::Refused("No build before this one is kept".into()));
        assert_eq!(outcome_of(Ok((500, b"oops".to_vec()))), Outcome::Transport("Fleet answered 500".into()));
        assert_eq!(outcome_of(Ok((202, br#"{"commit":"b"}"#.to_vec()))), Outcome::Started);
        assert_eq!(why_not(&outcome_of(Err("no connection in 2 seconds".into()))), "no connection in 2 seconds");
    }

    #[test]
    fn the_undo_is_the_build_befores_own_armada_beside_fleet_json() {
        assert_eq!(
            undo_path(Some("/Users/user")),
            Some(PathBuf::from("/Users/user/Library/Application Support/Armada/builds/previous/armada"))
        );
        assert_eq!(undo_path(None), None);
    }
}
