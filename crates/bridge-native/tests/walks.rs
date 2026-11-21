//! Walks played as the app plays them, headless (`--walk <name> --check`). gpui's Mac platform must be
//! made on the process's main thread, which a test thread is not, so each walk runs as its own process.

use std::process::Command;

fn plays(walk: &str) {
    let out = Command::new(env!("CARGO_BIN_EXE_bridge-native")).args(["--walk", walk, "--check"]).output().expect("the app runs");
    assert!(out.status.success(), "the {walk} walk did not play: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or_default());
}

/// The message box walk sends a numbered list of two long items and checks, at two widths, that the
/// bubble is drawn inside the thread rather than running past its left edge.
#[test]
fn a_long_list_in_a_message_wraps_inside_the_thread_at_two_widths() {
    plays("job-view-composer");
}

/// The tour checks the sketch and the plan in the thread stand inside the box's edges, on a pane
/// narrower than the column, where the rows' padding once let a card reach almost to the pane's edge.
#[test]
fn a_card_in_the_thread_keeps_the_box_column() {
    plays("jobs-screens");
}

/// A Job whose only wait is its plan: the band has no fold and opens no empty panel, and a press on it
/// opens the plan to review.
#[test]
fn needs_you_with_only_a_plan_opens_the_plan() {
    plays("needs-plan");
}

/// Two questions in Needs you, each one line that opens the review sheet on it, where a press or a number chooses and
/// Send answer sends.
#[test]
fn a_question_in_needs_you_is_one_line_and_the_sheet_answers_it() {
    plays("native-ways-buttons");
}

/// Switching between Jobs, a thread of six hundred items among them, with its scroll and its box's words
/// kept; the notes the walk emits carry each switch's time.
#[test]
fn switching_jobs_keeps_each_place() {
    plays("jobs-switching");
}

/// A value Fleet learned shows a Learned chip that takes the keys to its setting, and Fleet's line in a Job's thread
/// about how its work lands has a link that opens Settings on that project's How work lands.
#[test]
fn a_learned_value_and_the_thread_line_about_it_link_to_the_setting() {
    plays("native-learned-links");
}

/// The Fleet panel on Fleet's build report: Running, Waiting, Refresh and its sentence, Update to main, Keep.
#[test]
fn the_fleet_panel_says_what_runs_and_what_refresh_builds() {
    plays("build-switcher-live");
}

/// Offers for the preview stand under Waiting by their Job's title, and Refresh and Skip act on them.
#[test]
fn the_fleet_panel_names_each_offer_by_its_job() {
    plays("native-preview-offers");
}

/// Add a branch opens the branches on origin; a plus puts one above the search, and Swap follows.
#[test]
fn add_a_branch_opens_the_branches_on_origin() {
    plays("native-build-origin");
}

/// The Disk row at the panel's foot opens the section with its meter and floors.
#[test]
fn the_disk_row_opens_its_section() {
    plays("native-fleet-disk");
}

/// Two Jobs selected with ⇧↓ move into Storefront from the row's menu, and the Job's own menu takes one out to no project.
#[test]
fn two_selected_jobs_move_into_a_project_and_one_moves_out() {
    plays("native-move-job-to-project");
}

/// For you lists what waits on the Job's owner, what it is building and what is landing; the keys move over the rows,
/// Option Down moves a waiting row, Enter opens one and Backspace withdraws it.
#[test]
fn for_you_lists_three_sections_and_its_keys_move_open_and_withdraw_a_row() {
    plays("native-for-you");
}

/// The side chat: Side chat in the box's bar and ⌘J open the split, a question is answered with what the copy read, Tell the
/// Job marks it Told, Ask on selected words quotes them, ⌘W shuts the split and ⌘J brings every question back.
#[test]
fn the_side_chat_opens_asks_tells_and_comes_back_after_its_split_is_shut() {
    plays("native-side-chat");
}

/// A Job's page runs its own inline script, which reaches no handler of the kit's, no peer connection, and no press: its
/// click() and ⌘↵ on the kit's Rebase it start no Job, and the walk's own press on it does.
#[test]
fn a_pages_own_script_runs_and_cannot_press_for_him() {
    plays("native-page-scripts");
}
