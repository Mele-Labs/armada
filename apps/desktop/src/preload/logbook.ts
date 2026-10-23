// The Logbook page's members of the preload surface. Main does not handle these channels yet, so an invoke rejects
// until it does; the mock answers them.

import { ipcRenderer } from "electron";

import type { LogbookApi } from "../shared/api/logbook";
import { CHANNELS } from "../shared/bridge";

export const logbookPreload: LogbookApi = {
  readLogbook: () => ipcRenderer.invoke(CHANNELS.readLogbook),
  editLogbookEntry: (id, text) => ipcRenderer.invoke(CHANNELS.editLogbookEntry, id, text),
  pinLogbookEntry: (id, pinned) => ipcRenderer.invoke(CHANNELS.pinLogbookEntry, id, pinned),
  forgetLogbookEntry: (id) => ipcRenderer.invoke(CHANNELS.forgetLogbookEntry, id),
};
