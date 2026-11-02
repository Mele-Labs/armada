// The Job browser's channels: each names a Job and a Browser tab of it, and the pane's own window is read off the sender, never sent.

import { BrowserWindow, type IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import { CHOICES, SITE_PERMISSIONS, type Answer } from "../shared/site-permissions";
import type { JobBrowsers } from "./job-browsers";

const text = (value: unknown): value is string => typeof value === "string" && value !== "";
const number = (value: unknown): value is number => typeof value === "number" && Number.isFinite(value);
const permission = (value: unknown) => SITE_PERMISSIONS.find((one) => one.permission === value)?.permission;
const choice = (value: unknown) => CHOICES.find((one) => one.choice === value)?.choice;
const ANSWERS: readonly Answer[] = ["allow", "block", "dismiss"];
const GOES = ["back", "forward", "reload"] as const;

/** `known`: whether Fleet holds a Job by this id, so a window can make a browser only for a Job that is one. */
export function handleJobBrowsers({ ipc, browsers, known }: { ipc: IpcMain; browsers: JobBrowsers; known: (sessionId: string) => boolean }): void {
  const hostOf = (event: Electron.IpcMainInvokeEvent) => BrowserWindow.fromWebContents(event.sender);
  ipc.handle(CHANNELS.jobBrowserList, () => browsers.list());
  ipc.handle(CHANNELS.jobBrowserTabs, (_event, sessionId: unknown, tabIds: unknown) => {
    if (text(sessionId) && known(sessionId) && Array.isArray(tabIds) && tabIds.every(text)) browsers.tabs(sessionId, tabIds);
  });
  ipc.handle(CHANNELS.jobBrowserShow, (_event, sessionId: unknown, tabId: unknown) => {
    if (text(sessionId) && text(tabId)) browsers.show(sessionId, tabId);
  });
  ipc.handle(CHANNELS.jobBrowserPlace, (event, sessionId: unknown, tabId: unknown, bounds: unknown) => {
    const host = hostOf(event);
    if (text(sessionId) && text(tabId) && host !== null) browsers.place(host, sessionId, tabId, bounds);
  });
  ipc.handle(CHANNELS.jobBrowserUnplace, (event, sessionId: unknown, tabId: unknown) => {
    const host = hostOf(event);
    if (text(sessionId) && text(tabId) && host !== null) browsers.unplace(host, sessionId, tabId);
  });
  ipc.handle(CHANNELS.jobBrowserDrive, (_event, sessionId: unknown, tabId: unknown) => {
    if (text(sessionId) && text(tabId)) browsers.drive(sessionId, tabId);
  });
  ipc.handle(CHANNELS.jobBrowserGo, (_event, sessionId: unknown, tabId: unknown, to: unknown) => {
    const going = GOES.find((one) => one === to);
    if (text(sessionId) && text(tabId) && going !== undefined) browsers.go(sessionId, tabId, going);
  });
  ipc.handle(CHANNELS.jobBrowserNavigate, (_event, sessionId: unknown, tabId: unknown, url: unknown) => {
    if (text(sessionId) && text(tabId) && text(url)) browsers.navigate(sessionId, tabId, url);
  });
  ipc.handle(CHANNELS.jobBrowserToWindow, (_event, sessionId: unknown, tabId: unknown) => {
    if (text(sessionId) && text(tabId)) browsers.toWindow(sessionId, tabId);
  });
  ipc.handle(CHANNELS.jobBrowserFreeze, (_event, sessionId: unknown, tabId: unknown) => (text(sessionId) && text(tabId) ? browsers.freeze(sessionId, tabId) : null));
  ipc.handle(CHANNELS.jobBrowserThaw, (_event, sessionId: unknown, tabId: unknown) => {
    if (text(sessionId) && text(tabId)) browsers.thaw(sessionId, tabId);
  });
  ipc.handle(CHANNELS.jobBrowserHold, (_event, sessionId: unknown, tabId: unknown, x: unknown, y: unknown) =>
    text(sessionId) && text(tabId) && number(x) && number(y) ? browsers.hold(sessionId, tabId, x, y) : null,
  );
  ipc.handle(CHANNELS.jobBrowserAim, (_event, sessionId: unknown, tabId: unknown, x: unknown, y: unknown) =>
    text(sessionId) && text(tabId) && number(x) && number(y) ? browsers.aim(sessionId, tabId, x, y) : null,
  );
  ipc.handle(CHANNELS.jobBrowserNote, (_event, sessionId: unknown, tabId: unknown, said: unknown) =>
    text(sessionId) && text(tabId) && typeof said === "string" && said.trim() !== "" ? browsers.note(sessionId, tabId, said) : { ok: false, why: "not_understood" },
  );
  ipc.handle(CHANNELS.jobBrowserChoose, (_event, sessionId: unknown, tabId: unknown, host: unknown, which: unknown, chosen: unknown) => {
    const [perm, pick] = [permission(which), choice(chosen)];
    if (text(sessionId) && text(tabId) && text(host) && perm !== undefined && pick !== undefined) browsers.choose(sessionId, tabId, host, perm, pick);
  });
  ipc.handle(CHANNELS.jobBrowserDefault, (_event, sessionId: unknown, tabId: unknown, which: unknown, chosen: unknown) => {
    const [perm, pick] = [permission(which), choice(chosen)];
    if (text(sessionId) && text(tabId) && perm !== undefined && pick !== undefined) browsers.setDefault(sessionId, tabId, perm, pick);
  });
  ipc.handle(CHANNELS.jobBrowserAnswer, (_event, sessionId: unknown, tabId: unknown, answer: unknown) => {
    const said = ANSWERS.find((one) => one === answer);
    if (text(sessionId) && text(tabId) && said !== undefined) browsers.answer(sessionId, tabId, said);
  });
}
