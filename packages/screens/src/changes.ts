// What should change, gathered at the gate: the review's small fixes and the
// notes written in View. #907.
//
// **The list is the note.** Request changes sends one note, so the list and the
// typed words go out together and the Drone is never told in two places.

import { proseText, type DecisionChange } from "@armada/components";
import type { JobConfidence, WalkNote } from "@armada/protocol";

/** The review's small fixes, listed first. */
export function smallFixesOf(confidence: JobConfidence): DecisionChange[] {
  return confidence.small_fixes.map((fix, at) => ({
    id: `small-fix-${at}`,
    from: "Small fix",
    text: `${proseText(fix.finding)}: ${fix.why}`,
  }));
}

const WALKED = "walk-note-";

/**
 * A note made walking the Job's work, as a row of What should change. Listed
 * with the rest, and **never written into the note**: Fleet delivers it, and
 * marks it sent, so a second send-back does not hand it over again.
 */
export function walkedChange(note: WalkNote): DecisionChange {
  return { id: `${WALKED}${note.id}`, from: "From your walk", text: `${note.element}: ${note.said}` };
}

export const isWalked = (id: string): boolean => id.startsWith(WALKED);
export const walkedId = (id: string): string => id.slice(WALKED.length);

/** The note Request changes sends: every listed change, then the typed words. */
export function noteWithChanges(changes: readonly DecisionChange[], note: string): string {
  if (changes.length === 0) return note;
  const typed = note.trim();
  return [
    "What should change:",
    ...changes.map((change) => `- ${change.from}: ${change.text}`),
    ...(typed === "" ? [] : ["", typed]),
  ].join("\n");
}
