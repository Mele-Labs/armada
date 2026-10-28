// A Job's browser: one page the Job's agent and the person drive together, drawn in the side panel beside the Job's thread, or in
// a window of its own with the Job's thread beside the page, as the walk window draws it. **Proposed, not on the wire**: nothing in Fleet or main carries it yet, so only the mock fills this store
// (`mock/scenarios/job-browser.ts`), and where nothing fills it no pane and no reminder is drawn.

import { useSyncExternalStore, type ReactNode } from "react";

import { openInPanel } from "../shared-panel/store";

/** Who is driving the page: the agent through Armada's tools, or the person, who took it over. */
export type BrowserDriver = "agent" | "you";

/** A note the person pinned on part of the page, at that part's box within the page. */
export type BrowserPin = { id: string; url: string; on: string; note: string; box: { top: number; left: number; width: number; height: number } };

/** The Job reached for a browser outside Bridge: which one, when, on which page, and whether the person has told it to use the Job browser. */
export type BrowserReminder = { id: string; at: string; tool: "cmux" | "Playwright"; url: string; answered?: true };

export type JobBrowser = {
  /** The addresses visited, oldest first, and which of them shows: Back and Forward move along it. */
  visited: readonly string[];
  at: number;
  /** Bumped by Reload, so the page is drawn again. */
  loads: number;
  driver: BrowserDriver;
  /** In a window of its own, which the side panel gives way to. */
  windowed: boolean;
  pins: readonly BrowserPin[];
  reminders: readonly BrowserReminder[];
  /** The Job's thread, floating left of the side panel, is folded away. */
  threadHidden: boolean;
  /** **A mock's field**: what is drawn where the page goes, for an address. In the app the page is a view main places, which nothing does yet. */
  page?: (url: string) => ReactNode;
};

/** The address showing. */
export const urlOf = (browser: JobBrowser): string => browser.visited[browser.at] ?? "";

let held: Readonly<Record<string, JobBrowser>> = {};
const listeners = new Set<() => void>();

function edit(sessionId: string, change: (one: JobBrowser) => JobBrowser): void {
  const one = held[sessionId];
  if (one === undefined) return;
  held = { ...held, [sessionId]: change(one) };
  listeners.forEach((hear) => hear());
}

/** The Job's browser as it stands, or nothing where the Job has none. */
export function useJobBrowser(sessionId: string | undefined): JobBrowser | undefined {
  return useSyncExternalStore(
    (hear) => {
      listeners.add(hear);
      return () => listeners.delete(hear);
    },
    () => (sessionId === undefined ? undefined : held[sessionId]),
  );
}

/** The Job's browser, whole, in place of any it had. */
export function setJobBrowser(sessionId: string, browser: JobBrowser): void {
  held = { ...held, [sessionId]: browser };
  listeners.forEach((hear) => hear());
}

/** Take over, or hand back to the agent. */
export const toggleDriver = (sessionId: string) => edit(sessionId, (one) => ({ ...one, driver: one.driver === "agent" ? "you" : "agent" }));
/** A new address, which drops whatever Forward would have gone to. */
export const navigate = (sessionId: string, url: string) => edit(sessionId, (one) => ({ ...one, visited: [...one.visited.slice(0, one.at + 1), url], at: one.at + 1 }));
export const goBack = (sessionId: string) => edit(sessionId, (one) => ({ ...one, at: Math.max(0, one.at - 1) }));
export const goForward = (sessionId: string) => edit(sessionId, (one) => ({ ...one, at: Math.min(one.visited.length - 1, one.at + 1) }));
export const reload = (sessionId: string) => edit(sessionId, (one) => ({ ...one, loads: one.loads + 1 }));
export const showThread = (sessionId: string, shown: boolean) => edit(sessionId, (one) => ({ ...one, threadHidden: !shown }));
/** The Job browser in the side panel, out of any window it had. */
export function openBrowser(sessionId: string): void {
  edit(sessionId, (one) => ({ ...one, windowed: false }));
  openInPanel({ kind: "browser", sessionId });
}
/** The Job browser in a window of its own. The side panel it moved from is closed by its own button. */
export const browserToWindow = (sessionId: string) => edit(sessionId, (one) => ({ ...one, windowed: true }));
/** The window's own close: the page is put away, and the Job keeps it. */
export const closeBrowserWindow = (sessionId: string) => edit(sessionId, (one) => ({ ...one, windowed: false }));
export const pinNote = (sessionId: string, pin: BrowserPin) => edit(sessionId, (one) => ({ ...one, pins: [...one.pins, pin] }));
export const answerReminder = (sessionId: string, id: string) =>
  edit(sessionId, (one) => ({ ...one, reminders: one.reminders.map((each) => (each.id === id ? { ...each, answered: true as const } : each)) }));
export const remindOfBrowser = (sessionId: string, reminder: BrowserReminder) => edit(sessionId, (one) => ({ ...one, reminders: [...one.reminders, reminder] }));

/** The Jobs whose browser has a window of its own. */
export function useBrowserWindows(): readonly string[] {
  const all = useSyncExternalStore(
    (hear) => {
      listeners.add(hear);
      return () => listeners.delete(hear);
    },
    () => held,
  );
  return Object.keys(all).filter((id) => all[id]!.windowed);
}
