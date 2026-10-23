// The Logbook page's members of the preload surface. Main answers them from Fleet (`main/logbook-channels.ts`); the
// mock answers them in a walk.

import { ipcRenderer } from "electron";

import type { LogbookApi } from "../shared/api/logbook";
import { CHANNELS } from "../shared/bridge";

export const logbookPreload: LogbookApi = {
  readLogbook: () => ipcRenderer.invoke(CHANNELS.readLogbook),
  editLogbookEntry: (id, text) => ipcRenderer.invoke(CHANNELS.editLogbookEntry, id, text),
  lockLogbookEntry: (id, locked) => ipcRenderer.invoke(CHANNELS.lockLogbookEntry, id, locked),
  forgetLogbookEntry: (id) => ipcRenderer.invoke(CHANNELS.forgetLogbookEntry, id),
  tidyLogbook: () => ipcRenderer.invoke(CHANNELS.tidyLogbook),
};
