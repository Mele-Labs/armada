// Requests' six members of the preload surface. Each names a channel and no port; `setRequestCookie` is the one that carries
// a kept header's value, toward main and Fleet, and nothing here hands one back.

import { ipcRenderer } from "electron";

import type { RequestsApi } from "../shared/api/requests";
import { CHANNELS } from "../shared/bridge";

export const requestsPreload: RequestsApi = {
  sendRequest: (send) => ipcRenderer.invoke(CHANNELS.sendRequest, send),
  listRequestHosts: () => ipcRenderer.invoke(CHANNELS.listRequestHosts),
  setRequestCookie: (host, header, value) => ipcRenderer.invoke(CHANNELS.setRequestCookie, host, header, value),
  forgetRequestCookie: (host) => ipcRenderer.invoke(CHANNELS.forgetRequestCookie, host),
  listRequestEnvironments: (sessionId) => ipcRenderer.invoke(CHANNELS.listRequestEnvironments, sessionId),
  saveRequestEnvironments: (file) => ipcRenderer.invoke(CHANNELS.saveRequestEnvironments, file),
};
