// The Job browser's channels: each names a Job, and the pane's own window is read off the sender, never sent.

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
  ipc.handle(CHANNELS.jobBrowserShow, (_event, sessionId: unknown) => {
    if (text(sessionId) && known(sessionId)) browsers.show(sessionId);
  });
  ipc.handle(CHANNELS.jobBrowserPlace, (event, sessionId: unknown, bounds: unknown) => {
    const host = hostOf(event);
    if (text(sessionId) && host !== null) browsers.place(host, sessionId, bounds);
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
  ipc.handle(CHANNELS.jobBrowserAim, (_event, sessionId: unknown, x: unknown, y: unknown) =>
    text(sessionId) && number(x) && number(y) ? browsers.aim(sessionId, x, y) : null,
  );
  ipc.handle(CHANNELS.jobBrowserNote, (_event, sessionId: unknown, said: unknown) =>
    text(sessionId) && typeof said === "string" && said.trim() !== "" ? browsers.note(sessionId, said) : { ok: false, why: "not_understood" },
  );
  ipc.handle(CHANNELS.jobBrowserChoose, (_event, sessionId: unknown, host: unknown, which: unknown, chosen: unknown) => {
    const [perm, pick] = [permission(which), choice(chosen)];
    if (text(sessionId) && text(host) && perm !== undefined && pick !== undefined) browsers.choose(sessionId, host, perm, pick);
  });
  ipc.handle(CHANNELS.jobBrowserDefault, (_event, sessionId: unknown, which: unknown, chosen: unknown) => {
    const [perm, pick] = [permission(which), choice(chosen)];
    if (text(sessionId) && perm !== undefined && pick !== undefined) browsers.setDefault(sessionId, perm, pick);
  });
  ipc.handle(CHANNELS.jobBrowserAnswer, (_event, sessionId: unknown, answer: unknown) => {
    const said = ANSWERS.find((one) => one === answer);
    if (text(sessionId) && said !== undefined) browsers.answer(sessionId, said);
  });
}
