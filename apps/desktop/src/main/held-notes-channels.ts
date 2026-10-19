// A Job's held notes on the IPC seam: the walk window's panel lists, edits, removes and sends them, and its bar sends them
// before Approve. `docs/practices/capture-window.md`, *Held notes*. Out of `index.ts`, which is at the length the gate refuses.
//
// **No call names a Job.** Main reads it off the view the call came from, the panel's walk on screen or the bar's window, as
// `walkPanel.subject` does; a note is named by its own id, and only a note on that Job is reached.

import type { IpcMain, IpcMainInvokeEvent } from "electron";

import type { NoteElement, Outcome, SendSessionNotes } from "@armada/protocol";
import type { PanelHeldNote } from "@armada/studios/api";
import { CHANNELS } from "../shared/bridge";
import type { HeldNote, HeldNotes } from "./capture/held-notes";
import type { CaptureWindows } from "./capture/windows";

/** Nothing reached, because the call named nothing this view holds. */
const NOT_HELD: Outcome = { ok: false, why: "not_understood" };

/** The element a note is on, as a person names it: its accessible name, else its words, else its tag. */
function onOf(element: NoteElement): string {
  const words = (element.label ?? element.text).trim().replace(/\s+/g, " ");
  if (words === "") return element.tag;
  return `${element.tag} “${words.length > 48 ? `${words.slice(0, 47)}…` : words}”`;
}

export function shownOf(held: readonly HeldNote[]): PanelHeldNote[] {
  return held.map(({ id, at, note }) => ({ id, at, said: note.said, on: onOf(note.place.element), ...(note.crop === undefined ? {} : { crop: note.crop }) }));
}

/**
 * **The one place a Job's held notes are sent**: every one as one turn, then dropped, those alone. Failure leaves them all
 * held and says why. A Job holding none sends nothing, since Fleet refuses an empty turn.
 */
export async function sendHeld(store: HeldNotes, sessionId: string, send: (body: SendSessionNotes) => Promise<Outcome>): Promise<Outcome> {
  const notes = store.of(sessionId);
  if (notes.length === 0) return { ok: true };
  const outcome = await send({ session_id: sessionId, notes: notes.map((one) => one.note) });
  if (outcome.ok) store.drop(sessionId, notes.map((one) => one.id));
  return outcome;
}

type Hosts = {
  ipc: IpcMain;
  store: HeldNotes;
  captureWindows: CaptureWindows;
  send: (body: SendSessionNotes) => Promise<Outcome>;
};

/** The Job's held notes changed: the panel showing it is told the list, and the bar of the walk on screen is told again. */
export function tellHeld(store: HeldNotes, captureWindows: CaptureWindows, sessionId: string): void {
  for (const panel of captureWindows.panels()) {
    if (panel.isDestroyed() || captureWindows.panelFrom(panel)?.id !== sessionId) continue;
    panel.send(CHANNELS.walkPanelHeldChanged, shownOf(store.of(sessionId)));
  }
  captureWindows.heldChanged(sessionId);
}

export function handleHeldNotes({ ipc, store, captureWindows, send }: Hosts): void {
  const job = (event: IpcMainInvokeEvent): string | null => captureWindows.panelFrom(event.sender)?.id ?? null;
  const named = (value: unknown): value is string => typeof value === "string" && value !== "";
  ipc.handle(CHANNELS.walkPanelHeld, (event) => {
    const sessionId = job(event);
    return sessionId === null ? [] : shownOf(store.of(sessionId));
  });
  ipc.handle(CHANNELS.walkPanelEditNote, (event, id: unknown, said: unknown): Outcome => {
    const sessionId = job(event);
    if (sessionId === null || !named(id) || typeof said !== "string") return NOT_HELD;
    return store.edit(sessionId, id, said) ? { ok: true } : said.trim() === "" ? { ok: false, why: "empty_brief" } : NOT_HELD;
  });
  ipc.handle(CHANNELS.walkPanelRemoveNote, (event, id: unknown): Outcome => {
    const sessionId = job(event);
    return sessionId !== null && named(id) && store.remove(sessionId, id) ? { ok: true } : NOT_HELD;
  });
  ipc.handle(CHANNELS.walkPanelSendNotes, async (event) => {
    const sessionId = job(event);
    return sessionId === null ? NOT_HELD : await sendHeld(store, sessionId, send);
  });
  // The bar's, before Approve: the Job its window's notes are held on.
  ipc.handle(CHANNELS.captureWindowSendNotes, async (event) => {
    const sessionId = captureWindows.from(event.sender)?.notesOn;
    return sessionId === undefined ? NOT_HELD : await sendHeld(store, sessionId, send);
  });
}
