// A Job's walk in a pane: the members of the preload surface it needs (`main/capture/job-walks.ts`). Each names a Job and
// the address of a walk it showed; the page and the bar are views main holds, and nothing of either crosses here but a picture.

import { ipcRenderer } from "electron";
import type { PanelHeldNote } from "@armada/studios/api";

import type { JobWalkApi } from "../shared/api/job-walk";
import { CHANNELS } from "../shared/bridge";

export const jobWalkPreload: Required<JobWalkApi> = {
  jobWalk: {
    place: (sessionId, walkId, bounds) =>
      ipcRenderer.invoke(CHANNELS.jobWalkPlace, sessionId, walkId, bounds),
    unplace: (sessionId, walkId) => ipcRenderer.invoke(CHANNELS.jobWalkUnplace, sessionId, walkId),
    freeze: (sessionId, walkId) => ipcRenderer.invoke(CHANNELS.jobWalkFreeze, sessionId, walkId),
    thaw: (sessionId, walkId) => ipcRenderer.invoke(CHANNELS.jobWalkThaw, sessionId, walkId),
    // The Job's notes, told as they change and asked for once, as the panel's are.
    held: (sessionId, onChanged) => {
      let pushed = false;
      const handler = (_event: unknown, told: string, notes: PanelHeldNote[]): void => {
        if (told !== sessionId) return;
        pushed = true;
        onChanged(notes);
      };
      ipcRenderer.on(CHANNELS.jobWalkHeldChanged, handler);
      void ipcRenderer.invoke(CHANNELS.jobWalkHeld, sessionId).then((notes: PanelHeldNote[]) => {
        if (!pushed) onChanged(notes);
      });
      return () => ipcRenderer.removeListener(CHANNELS.jobWalkHeldChanged, handler);
    },
    editNote: (sessionId, id, said) =>
      ipcRenderer.invoke(CHANNELS.jobWalkEditNote, sessionId, id, said),
    removeNote: (sessionId, id) => ipcRenderer.invoke(CHANNELS.jobWalkRemoveNote, sessionId, id),
    sendNotes: (sessionId) => ipcRenderer.invoke(CHANNELS.jobWalkSendNotes, sessionId),
  },
};
