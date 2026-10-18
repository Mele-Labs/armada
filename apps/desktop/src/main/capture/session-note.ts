// What a Session is told when the person approves a page it showed, and what a note in a pull request's
// walk becomes where no Session holds it. A note to a Session is `send_session_note`'s body, `pinned-note.ts`.
// **Of several takes, the one on screen is named**, `B, Needs you first`, so the Session knows which page it is about.

import type { StudioCapture } from "@armada/protocol";

import type { ReviewWalkNote } from "../../shared/review-walk";
import { chainOf } from "./bounds";
import type { ReviewWalkOwner } from "./layout";

/** The take on screen: its letter on the bar and the name the Session gave it. */
export type Take = { letter: string; name: string };

/**
 * A note taken in a pull request's walk window no Session holds the pull request of. **It stays in Armada** as a note on the
 * pull request: never a comment on the forge, not even a draft one. With a holder the window sends it as a pinned note.
 */
export function reviewNoteOf(review: ReviewWalkOwner, said: string, capture: StudioCapture, now: string): ReviewWalkNote {
  return { manifestId: review.manifestId, number: review.number, at: now, text: said.trim(), location: `${chainOf(capture)}, ${capture.location}` };
}

/** What Approve sends. **Fleet reads the walk's end off this**: `walking.rs`'s `approved_address`. */
export function approvedText(address: string, take?: Take): string {
  return `Approved${take === undefined ? "" : ` ${take.letter}, ${take.name}`}: ${address}`;
}
