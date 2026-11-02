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
    onCapture: (listener) => {
      const handler = (_event: unknown, sessionId: string, tabId: string): void =>
        listener(sessionId, tabId);
      ipcRenderer.on(CHANNELS.jobBrowserCapture, handler);
      return () => void ipcRenderer.removeListener(CHANNELS.jobBrowserCapture, handler);
    },
    tabs: (sessionId, tabIds) => ipcRenderer.invoke(CHANNELS.jobBrowserTabs, sessionId, tabIds),
    show: (sessionId, tabId) => ipcRenderer.invoke(CHANNELS.jobBrowserShow, sessionId, tabId),
    place: (sessionId, tabId, bounds) =>
      ipcRenderer.invoke(CHANNELS.jobBrowserPlace, sessionId, tabId, bounds),
    unplace: (sessionId, tabId) => ipcRenderer.invoke(CHANNELS.jobBrowserUnplace, sessionId, tabId),
    drive: (sessionId, tabId) => ipcRenderer.invoke(CHANNELS.jobBrowserDrive, sessionId, tabId),
    go: (sessionId, tabId, to) => ipcRenderer.invoke(CHANNELS.jobBrowserGo, sessionId, tabId, to),
    navigate: (sessionId, tabId, url) =>
      ipcRenderer.invoke(CHANNELS.jobBrowserNavigate, sessionId, tabId, url),
    toWindow: (sessionId, tabId) =>
      ipcRenderer.invoke(CHANNELS.jobBrowserToWindow, sessionId, tabId),
    freeze: (sessionId, tabId) => ipcRenderer.invoke(CHANNELS.jobBrowserFreeze, sessionId, tabId),
    thaw: (sessionId, tabId) => ipcRenderer.invoke(CHANNELS.jobBrowserThaw, sessionId, tabId),
    hold: (sessionId, tabId, x, y) =>
      ipcRenderer.invoke(CHANNELS.jobBrowserHold, sessionId, tabId, x, y),
    aim: (sessionId, tabId, x, y) =>
      ipcRenderer.invoke(CHANNELS.jobBrowserAim, sessionId, tabId, x, y),
    note: (sessionId, tabId, said) =>
      ipcRenderer.invoke(CHANNELS.jobBrowserNote, sessionId, tabId, said),
    choose: (sessionId, tabId, host, permission, choice) =>
      ipcRenderer.invoke(CHANNELS.jobBrowserChoose, sessionId, tabId, host, permission, choice),
    setDefault: (sessionId, tabId, permission, choice) =>
      ipcRenderer.invoke(CHANNELS.jobBrowserDefault, sessionId, tabId, permission, choice),
    answer: (sessionId, tabId, answer) =>
      ipcRenderer.invoke(CHANNELS.jobBrowserAnswer, sessionId, tabId, answer),
  },
};
