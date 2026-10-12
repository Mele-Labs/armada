// The keys live on an open Studio — #1364. Read off `actions.toml` through the
// generated registry, never written down here.
//
// **A place rather than an object, which is what `open studio` scopes.** The
// Board's map acts on the row under the cursor and job detail's on the job
// open; these act on the board a person is looking at, so they are their own
// small map rather than a branch inside either.
//
// The safety rules are `keys.ts`'s and are imported: a single key is suppressed
// while a text input holds focus, so typing "seven notes" into a note does not
// open three more fields, and a modifier means another tier is being addressed.

import { useEffect } from "react";
import { isPressed, type Press, type StudioNodeByHandKind } from "@armada/components";

import { holdsText } from "@armada/screens/src/keys";

/** Each kind's binding, from the registry. The one place a key is written is there. */
const ADD: readonly (readonly [string, StudioNodeByHandKind])[] = [
  ["add_note", "note"],
  ["add_link", "link"],
  ["add_sketch", "sketch"],
  ["add_zone", "zone"],
];

/** What a press means on an open Studio, or `null` where it means nothing here. */
export function addPressOf(event: Press & { target: EventTarget | null }): StudioNodeByHandKind | null {
  if (holdsText(event.target)) return null;
  return ADD.find(([id]) => isPressed(id, event))?.[1] ?? null;
}

/**
 * `N`, `V` and `S` on an open Studio, while it can be written to.
 *
 * **Bound only while `editable`**, so a Studio reopened read-only answers no
 * key — the same rule its controls draw, rather than a second one.
 */
export function useAddNodeKeys(editable: boolean, onAdding: (kind: StudioNodeByHandKind) => void): void {
  useEffect(() => {
    if (!editable) return;
    function pressed(event: KeyboardEvent): void {
      const kind = addPressOf(event);
      if (kind === null) return;
      event.preventDefault();
      onAdding(kind);
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, [editable, onAdding]);
}

/** Run's binding, from the registry. */

/**
 * `R` on an open Studio opens the rail's Run menu — the owner, 2 Oct 2026.
 *
 * **Bound only while Run is on**, so the key is dead exactly when the rail
 * draws Run off: read-only, no connection, or nothing declared to run.
 */
export function useRunKey(on: boolean, onRun: () => void): void {
  useEffect(() => {
    if (!on) return;
    function pressed(event: KeyboardEvent): void {
      if (holdsText(event.target)) return;
      if (!isPressed("start_studio_run", event)) return;
      event.preventDefault();
      onRun();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, [on, onRun]);
}
