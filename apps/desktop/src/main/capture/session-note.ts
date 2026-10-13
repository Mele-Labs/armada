// What a Session is told when the person takes a note on a page it showed, or approves it.
//
// **The person's words, then where they pointed**: one line the Session can act on without
// the markup, the styles or the selector, which stay in main as every capture's do.
// **Of several takes, the one on screen is named first**, `B, Needs you first`, so the Session
// knows which page it is about.

import type { StudioCapture } from "@armada/protocol";
import { chainOf } from "./bounds";

/** The take on screen: its letter on the bar and the name the Session gave it. */
export type Take = { letter: string; name: string };

export function sessionNoteText(said: string, capture: StudioCapture, address: string, take?: Take): string {
  const on = take === undefined ? "" : `${take.letter}, ${take.name}: `;
  return `${said.trim()}\nOn ${on}${chainOf(capture)}, ${capture.location}, at ${address}`;
}

/** What Approve sends. **Fleet reads the walk's end off this**: `walking.rs`'s `approved_address`. */
export function approvedText(address: string, take?: Take): string {
  return `Approved${take === undefined ? "" : ` ${take.letter}, ${take.name}`}: ${address}`;
}
