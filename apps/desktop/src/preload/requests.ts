// Requests' five members of the preload surface. Each names a channel and no port; `saveRequestEnvironment` is the one that
// carries a secret's value, toward main and Fleet, and nothing here hands one back.

import { ipcRenderer } from "electron";

import type { RequestsApi } from "../shared/api/requests";
import { CHANNELS } from "../shared/bridge";

export const requestsPreload: RequestsApi = {
  readRequests: (sessionId, repository) => ipcRenderer.invoke(CHANNELS.readRequests, sessionId, repository),
  saveRequestCollection: (collection, sessionId, repository) => ipcRenderer.invoke(CHANNELS.saveRequestCollection, collection, sessionId, repository),
  saveRequestEnvironment: (environment, sessionId, repository) => ipcRenderer.invoke(CHANNELS.saveRequestEnvironment, environment, sessionId, repository),
  deleteRequestEnvironment: (name, sessionId, repository) => ipcRenderer.invoke(CHANNELS.deleteRequestEnvironment, name, sessionId, repository),
  sendRequest: (send) => ipcRenderer.invoke(CHANNELS.sendRequest, send),
};
