// A Job's walk in a pane, on the IPC seam: where its pane is, and the notes held on its Job. `docs/practices/capture-window.md`, *A Job's walk as a tab*.
//
// **Each call names a Job and the address of a walk it showed**, and main acts only on a walk it holds: the page and the bar are main's, and the
// window the pane is in is read off the sender, never sent. The Job's notes are the ones `held-notes-channels.ts` sends, and only that Job's are reached.

import { BrowserWindow, type IpcMain } from "electron";

import type { Outcome, SendSessionNotes } from "@armada/protocol";
import { CHANNELS } from "../shared/bridge";
import type { HeldNotes } from "./capture/held-notes";
import type { CaptureWindows } from "./capture/windows";
import { sendHeld, shownOf } from "./held-notes-channels";

const text = (value: unknown): value is string => typeof value === "string" && value !== "";

/** Nothing reached, because the call named nothing main holds. */
const NOT_HELD: Outcome = {
  ok: false,
  why: "not_understood",
};

type Hosts = {
  ipc: IpcMain;
  store: HeldNotes;
  captureWindows: CaptureWindows;
  send: (body: SendSessionNotes) => Promise<Outcome>;
};

export function handleJobWalks({ ipc, store, captureWindows, send }: Hosts): void {
  ipc.handle(
    CHANNELS.jobWalkPlace,
    (event, sessionId: unknown, walkId: unknown, bounds: unknown) => {
      const host = BrowserWindow.fromWebContents(event.sender);
      if (text(sessionId) && text(walkId) && host !== null)
        captureWindows.place(host, sessionId, walkId, bounds);
    },
  );
  ipc.handle(CHANNELS.jobWalkUnplace, (_event, sessionId: unknown, walkId: unknown) => {
    if (text(sessionId) && text(walkId)) captureWindows.unplace(sessionId, walkId);
  });
  ipc.handle(CHANNELS.jobWalkFreeze, (_event, sessionId: unknown, walkId: unknown) =>
    text(sessionId) && text(walkId) ? captureWindows.freeze(sessionId, walkId) : null,
  );
  ipc.handle(CHANNELS.jobWalkThaw, (_event, sessionId: unknown, walkId: unknown) => {
    if (text(sessionId) && text(walkId)) captureWindows.thaw(sessionId, walkId);
  });
  ipc.handle(CHANNELS.jobWalkHeld, (_event, sessionId: unknown) =>
    text(sessionId) ? shownOf(store.of(sessionId)) : [],
  );
  ipc.handle(
    CHANNELS.jobWalkEditNote,
    (_event, sessionId: unknown, id: unknown, said: unknown): Outcome => {
      if (!text(sessionId) || !text(id) || typeof said !== "string") return NOT_HELD;
      return store.edit(sessionId, id, said)
        ? {
            ok: true,
          }
        : said.trim() === ""
          ? {
              ok: false,
              why: "empty_brief",
            }
          : NOT_HELD;
    },
  );
  ipc.handle(
    CHANNELS.jobWalkRemoveNote,
    (_event, sessionId: unknown, id: unknown): Outcome =>
      text(sessionId) && text(id) && store.remove(sessionId, id)
        ? {
            ok: true,
          }
        : NOT_HELD,
  );
  ipc.handle(CHANNELS.jobWalkSendNotes, async (_event, sessionId: unknown) =>
    text(sessionId) ? await sendHeld(store, sessionId, send) : NOT_HELD,
  );
}

/** The Job's held notes changed: every window of Bridge's is told the list, which the Job's thread shows. */
export function tellJobWalkHeld(store: HeldNotes, sessionId: string): void {
  const notes = shownOf(store.of(sessionId));
  for (const window of BrowserWindow.getAllWindows())
    if (!window.isDestroyed())
      window.webContents.send(CHANNELS.jobWalkHeldChanged, sessionId, notes);
}
