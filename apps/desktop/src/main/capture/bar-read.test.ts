import { beforeEach, expect, it, vi } from "vitest";

import { CHANNELS } from "../../shared/bridge";
import type { CaptureWindowState } from "../../shared/capture-window";

const listeners = new Map<string, (event: unknown, state: CaptureWindowState) => void>();
let answer: CaptureWindowState | null = null;
type BarApi = { read: (onChanged: (state: CaptureWindowState) => void) => () => void };
let exposed: { captureWindow: BarApi } | undefined;

vi.mock("electron", () => ({
  contextBridge: { exposeInMainWorld: (name: string, api: never) => void (name === "armada" ? (exposed = api) : undefined) },
  ipcRenderer: {
    on: (channel: string, handler: (event: unknown, state: CaptureWindowState) => void) => listeners.set(channel, handler),
    removeListener: (channel: string) => listeners.delete(channel),
    invoke: async (channel: string) => (channel === CHANNELS.captureWindowRead ? answer : undefined),
    send: () => {},
  },
  webUtils: {},
}));

const state = (name: string): CaptureWindowState => ({ served: { run: "r", name, address: "a" } }) as CaptureWindowState;

beforeEach(() => {
  listeners.clear();
  answer = null;
  vi.resetModules();
});

async function bar(): Promise<BarApi> {
  await import("../../preload/index");
  return exposed!.captureWindow;
}

// Main pushes on `did-finish-load`, which can land before the bar's listener does: the walk window's
// bar then drew nothing, since the one pull it made threw its answer away.
it("draws the state main holds when the push came before the listener", async () => {
  answer = state("held");
  const seen: CaptureWindowState[] = [];
  (await bar()).read((one) => seen.push(one));
  await new Promise((done) => setTimeout(done, 0));
  expect(seen.map((one) => one.served.name)).toEqual(["held"]);
});

it("does not draw the pulled state over a push that arrived first", async () => {
  answer = state("old");
  const seen: CaptureWindowState[] = [];
  (await bar()).read((one) => seen.push(one));
  listeners.get(CHANNELS.captureWindowChanged)?.({}, state("new"));
  await new Promise((done) => setTimeout(done, 0));
  expect(seen.map((one) => one.served.name)).toEqual(["new"]);
});
