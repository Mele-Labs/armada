// Teaching's members of the preload surface: one request each, and the renderer names no port.

import { ipcRenderer } from "electron";

import type { TeachingApi } from "../shared/api/teaching";
import { CHANNELS } from "../shared/bridge";

export const teachingPreload: TeachingApi = {
  readTeaching: (read) => ipcRenderer.invoke(CHANNELS.readTeaching, read),
  saveTeaching: (save) => ipcRenderer.invoke(CHANNELS.saveTeaching, save),
  listWordRules: (repository, sessionId) => ipcRenderer.invoke(CHANNELS.listWordRules, repository, sessionId),
};
