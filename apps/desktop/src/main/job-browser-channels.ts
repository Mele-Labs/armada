// The Job browser's channels: each names a Job, and the pane's own window is read off the sender, never sent.

import { BrowserWindow, type IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { JobBrowsers } from "./job-browsers";

const text = (value: unknown): value is string => typeof value === "string" && value !== "";
const number = (value: unknown): value is number => typeof value === "number" && Number.isFinite(value);
const GOES = ["back", "forward", "reload"] as const;

export function handleJobBrowsers({ ipc, browsers }: { ipc: IpcMain; browsers: JobBrowsers }): void {
  const hostOf = (event: Electron.IpcMainInvokeEvent) => BrowserWindow.fromWebContents(event.sender);
  ipc.handle(CHANNELS.jobBrowserList, () => browsers.list());
  ipc.handle(CHANNELS.jobBrowserShow, (_event, sessionId: unknown) => {
    if (text(sessionId)) browsers.show(sessionId);
  });
  ipc.handle(CHANNELS.jobBrowserPlace, (event, sessionId: unknown, bounds: unknown) => {
    const host = hostOf(event);
    if (text(sessionId) && host !== null) browsers.place(host, sessionId, bounds as never);
  });
  ipc.handle(CHANNELS.jobBrowserUnplace, (event, sessionId: unknown) => {
    const host = hostOf(event);
    if (text(sessionId) && host !== null) browsers.unplace(host, sessionId);
  });
  ipc.handle(CHANNELS.jobBrowserDrive, (_event, sessionId: unknown) => {
    if (text(sessionId)) browsers.drive(sessionId);
  });
  ipc.handle(CHANNELS.jobBrowserGo, (_event, sessionId: unknown, to: unknown) => {
    const going = GOES.find((one) => one === to);
    if (text(sessionId) && going !== undefined) browsers.go(sessionId, going);
  });
  ipc.handle(CHANNELS.jobBrowserNavigate, (_event, sessionId: unknown, url: unknown) => {
    if (text(sessionId) && text(url)) browsers.navigate(sessionId, url);
  });
  ipc.handle(CHANNELS.jobBrowserToWindow, (_event, sessionId: unknown) => {
    if (text(sessionId)) browsers.toWindow(sessionId);
  });
  ipc.handle(CHANNELS.jobBrowserFreeze, (_event, sessionId: unknown) => (text(sessionId) ? browsers.freeze(sessionId) : null));
  ipc.handle(CHANNELS.jobBrowserThaw, (_event, sessionId: unknown) => {
    if (text(sessionId)) browsers.thaw(sessionId);
  });
  ipc.handle(CHANNELS.jobBrowserHold, (_event, sessionId: unknown, x: unknown, y: unknown) =>
    text(sessionId) && number(x) && number(y) ? browsers.hold(sessionId, x, y) : null,
  );
  ipc.handle(CHANNELS.jobBrowserNote, (_event, sessionId: unknown, said: unknown) =>
    text(sessionId) && typeof said === "string" && said.trim() !== "" ? browsers.note(sessionId, said) : { ok: false, why: "not_understood" },
  );
}
