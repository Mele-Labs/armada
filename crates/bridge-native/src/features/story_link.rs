//! The ask a press on a Story link leaves for the Job it opens. The Story tab of that Job takes it with [`take`] when
//! it draws, and puts that ask in front.

use gpui::{App, Global};
use ipc::SessionId;

/// The ask to show, and the Job it is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoryRequest {
    pub session_id: SessionId,
    pub ask_id: String,
}

#[derive(Default)]
struct Pending(Option<StoryRequest>);

impl Global for Pending {}

/// Leave a request, in place of any that nobody took.
pub fn put(request: StoryRequest, cx: &mut App) {
    cx.default_global::<Pending>().0 = Some(request);
}

/// Take the request, which leaves none.
pub fn take(cx: &mut App) -> Option<StoryRequest> {
    cx.default_global::<Pending>().0.take()
}

/// The request left, if nobody has taken it, which stays.
pub fn peek(cx: &App) -> Option<StoryRequest> {
    cx.try_global::<Pending>().and_then(|pending| pending.0.clone())
}
