// What a Session is told when the person takes a note on a page it showed.
//
// **The person's words, then where they pointed**: one line the Session can act on without
// the markup, the styles or the selector, which stay in main as every capture's do.

import type { StudioCapture } from "@armada/protocol";
import { chainOf } from "./bounds";

export function sessionNoteText(said: string, capture: StudioCapture, address: string): string {
  return `${said.trim()}\nOn ${chainOf(capture)}, ${capture.location}, at ${address}`;
}
