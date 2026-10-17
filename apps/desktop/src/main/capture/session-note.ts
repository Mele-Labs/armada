// What a Session is told when the person takes a note on a page it showed, or approves it.
//
// **The person's words, then where they pointed**: one line the Session can act on without
// the markup, the styles or the selector, which stay in main as every capture's do.
// **Of several takes, the one on screen is named first**, `B, Needs you first`, so the Session
// knows which page it is about.

import type { StudioCapture } from "@armada/protocol";

import type { ReviewWalkNote } from "../../shared/review-walk";
import { chainOf } from "./bounds";
import type { ReviewWalkOwner } from "./layout";

/** The take on screen: its letter on the bar and the name the Session gave it. */
export type Take = { letter: string; name: string };

export function sessionNoteText(said: string, capture: StudioCapture, address: string, take?: Take): string {
  const on = take === undefined ? "" : `${take.letter}, ${take.name}: `;
  return `${said.trim()}\nOn ${on}${chainOf(capture)}, ${capture.location}, at ${address}`;
}

/**
 * What becomes of a note taken in a pull request's walk window. **With a Session holding the pull request it is that Session's
 * message**, naming the pull request and the place it points at. **Without one it stays in Armada** as a note on the pull request:
 * never a comment on the forge, not even a draft one.
 */
export function reviewNoteOf(
  review: ReviewWalkOwner,
  said: string,
  capture: StudioCapture,
  now: string,
): { toSession: { id: string; text: string } } | { kept: ReviewWalkNote } {
  if (review.holder !== undefined) {
    return { toSession: { id: review.holder.id, text: `Walking ${review.title}\n${sessionNoteText(said, capture, review.address)}` } };
  }
  return { kept: { manifestId: review.manifestId, number: review.number, at: now, text: said.trim(), location: `${chainOf(capture)}, ${capture.location}` } };
}

/** What Approve sends. **Fleet reads the walk's end off this**: `walking.rs`'s `approved_address`. */
export function approvedText(address: string, take?: Take): string {
  return `Approved${take === undefined ? "" : ` ${take.letter}, ${take.name}`}: ${address}`;
}
