//! The walk of the failure toasts and the held "Fleet is slow" notice, on the real toast shelf. Each step's scene
//! raises what a Fleet fault would, so the owner sees what he otherwise sees only when something fails.

use super::{answer, Step};
use gpui::App;

const TALL: &str = "Your answer to Rate limits per token have no default did not reach Fleet because the connection dropped while it was being sent, and the sentence runs over several lines of the card, so it is taller than the one in front";

pub static STEPS: [Step; 4] = [
    answer("One failure toast, in the corner: no answer in 30 seconds. It stays until he closes it", "toasts-one", "alert", "Toast: no answer in 30 seconds"),
    answer("Two failure toasts stacked: the back one is taller, and peeks out by its step only, not its whole height", "toasts-two", "alert", "Toast: Short one in front"),
    answer("Fleet is slow stands alone: two more requests timed out, and neither raised a toast of its own", "toasts-slow", "alert", "Toast: Fleet is slow"),
    answer("Fleet answers: the notice clears and the corner is empty", "toasts-clear", "button", "Toasts"),
];

/// What a step of this walk raises, from nothing told: each scene starts the shelf over.
pub fn scene(name: &str, cx: &mut App) -> Result<(), String> {
    use crate::toasts::{at_rest, fleet, tell};
    at_rest(cx);
    match name {
        "one" => tell("no answer in 30 seconds", cx),
        "two" => {
            tell(TALL, cx);
            tell("Short one in front", cx);
        }
        "slow" => {
            fleet(vec![crate::titlebar::slow_notice(true).expect("slow")], cx);
            tell("no answer in 30 seconds", cx);
            tell("Your answer did not reach Fleet: no answer in 30 seconds", cx);
        }
        "clear" => {
            fleet(crate::titlebar::slow_notice(false).into_iter().collect(), cx);
        }
        other => return Err(format!("no toasts scene named {other}")),
    }
    Ok(())
}
