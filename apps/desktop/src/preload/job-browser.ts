// The Job browser's members of the preload surface (`main/job-browser-channels.ts`). Each names a Job; the page itself
// is a view main holds, and nothing of it crosses here but its state, its picture under Capture and the part picked.

import { ipcRenderer } from "electron";

import type { JobBrowserApi, JobBrowserState } from "../shared/api/job-browser";
import { CHANNELS } from "../shared/bridge";

function hear<T>(channel: string, listener: (value: T) => void): () => void {
  const handler = (_event: unknown, value: T): void => listener(value);
  ipcRenderer.on(channel, handler);
  return () => void ipcRenderer.removeListener(channel, handler);
}

export const jobBrowserPreload: Required<JobBrowserApi> = {
  jobBrowser: {
    list: () => ipcRenderer.invoke(CHANNELS.jobBrowserList),
    onChanged: (listener) => hear<JobBrowserState>(CHANNELS.jobBrowserChanged, listener),
    onShow: (listener) => hear<string>(CHANNELS.jobBrowserShown, listener),
    onCapture: (listener) => hear<string>(CHANNELS.jobBrowserCapture, listener),
    show: (sessionId) => ipcRenderer.invoke(CHANNELS.jobBrowserShow, sessionId),
    place: (sessionId, bounds) => ipcRenderer.invoke(CHANNELS.jobBrowserPlace, sessionId, bounds),
    unplace: (sessionId) => ipcRenderer.invoke(CHANNELS.jobBrowserUnplace, sessionId),
    drive: (sessionId) => ipcRenderer.invoke(CHANNELS.jobBrowserDrive, sessionId),
    go: (sessionId, to) => ipcRenderer.invoke(CHANNELS.jobBrowserGo, sessionId, to),
    navigate: (sessionId, url) => ipcRenderer.invoke(CHANNELS.jobBrowserNavigate, sessionId, url),
    toWindow: (sessionId) => ipcRenderer.invoke(CHANNELS.jobBrowserToWindow, sessionId),
    freeze: (sessionId) => ipcRenderer.invoke(CHANNELS.jobBrowserFreeze, sessionId),
    thaw: (sessionId) => ipcRenderer.invoke(CHANNELS.jobBrowserThaw, sessionId),
    hold: (sessionId, x, y) => ipcRenderer.invoke(CHANNELS.jobBrowserHold, sessionId, x, y),
    note: (sessionId, said) => ipcRenderer.invoke(CHANNELS.jobBrowserNote, sessionId, said),
  },
};
