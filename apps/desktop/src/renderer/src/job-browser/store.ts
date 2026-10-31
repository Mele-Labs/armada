// A Job's browser: one page the Job's agent and the person drive together, drawn in the side panel beside the Job's thread, or in
// a window of its own with the Job's thread beside the page, as the walk window draws it. **Main holds the page** (`main/job-browsers.ts`)
// and tells this store what it shows; the acts here go to main. The mock has no `jobBrowser` on its bridge, so there this store plays
// the browser itself over a page drawn from fixtures (`mock/scenarios/job-browser.ts`).

import { useSyncExternalStore, type ReactNode } from "react";

import type { JobBrowserApi, JobBrowserState } from "../../../shared/api/job-browser";
import { effectiveChoice, NO_SETTINGS, type Choice, type SitePermission, type SiteSettings } from "../../../shared/site-permissions";
import { viewingNow } from "../cockpit/viewed";
import { openInPanel } from "../shared-panel/store";

/** Who is driving the page: the agent through Armada's tools, or the person, who took it over. */
export type BrowserDriver = "agent" | "you";

/** A note the person pinned on part of the page, at that part's box within the page. Only the mock draws them. */
export type BrowserPin = { id: string; url: string; on: string; note: string; box: { top: number; left: number; width: number; height: number } };

export type JobBrowser = {
  url: string;
  title: string;
  back: boolean;
  forward: boolean;
  /** Bumped by Reload, so the mock's page is drawn again. */
  loads: number;
  driver: BrowserDriver;
  /** In a window of its own, which the side panel gives way to. */
  windowed: boolean;
  pins: readonly BrowserPin[];
  /** The Job's thread, floating left of the side panel, is folded away. */
  threadHidden: boolean;
  site: SiteSettings;
  /** **The mock's**: what is drawn where the page goes. In the app the page is a view main places over the pane. */
  page?: (url: string, sessionId: string) => ReactNode;
};

/** Main's, where the bridge has one: the browser is real. */
type Wired = NonNullable<JobBrowserApi["jobBrowser"]>;
let wired: Wired | undefined;

let held: Readonly<Record<string, JobBrowser>> = {};
/** The mock's history for each Job, which main keeps for the app. */
const visits = new Map<string, { visited: string[]; at: number }>();
const listeners = new Set<() => void>();
/** Jobs whose browser the person pressed open, which main answers with a show. */
const asked = new Set<string>();

function told(): void {
  listeners.forEach((hear) => hear());
}

function edit(sessionId: string, change: (one: JobBrowser) => JobBrowser): void {
  const one = held[sessionId];
  if (one === undefined) return;
  held = { ...held, [sessionId]: change(one) };
  told();
}

/** The address showing. */
export const urlOf = (browser: JobBrowser): string => browser.url;

/** The bridge this document talks to, once: what main holds is read, and each change after it is folded in. */
export function wireJobBrowsers(api: Wired): () => void {
  wired = api;
  const fold = (state: JobBrowserState) => {
    const was = held[state.sessionId];
    held = {
      ...held,
      [state.sessionId]: {
        url: state.url,
        title: state.title,
        back: state.back,
        forward: state.forward,
        loads: was?.loads ?? 0,
        driver: state.driver === "owner" ? "you" : "agent",
        windowed: state.windowed,
        pins: was?.pins ?? [],
        threadHidden: was?.threadHidden ?? false,
        site: state.site,
      },
    };
    told();
  };
  void api.list().then((all) => all.forEach(fold));
  const offChanged = api.onChanged(fold);
  // An agent's open reaches here as a press does. Only the person's press, or a Job he has open, opens the panel: the rest wait behind the Job's Browser tile.
  const offShow = api.onShow((sessionId) => {
    if (asked.delete(sessionId) || viewingNow(sessionId)) openInPanel({ kind: "browser", sessionId });
  });
  return () => {
    offChanged();
    offShow();
    wired = undefined;
  };
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

/** Main's own calls, where this document has them: the pane reports the page's place and holds it still under Capture. */
export const jobBrowserApi = (): Wired | undefined => wired;

/** **The mock's**: a Job's browser on its first addresses, the last showing. */
export function seedJobBrowser(sessionId: string, seed: { visited: string[]; driver?: BrowserDriver; page: (url: string, sessionId: string) => ReactNode }): void {
  visits.set(sessionId, { visited: [...seed.visited], at: seed.visited.length - 1 });
  held = { ...held, [sessionId]: { url: "", title: "", back: false, forward: false, loads: 0, driver: seed.driver ?? "agent", windowed: false, pins: [], threadHidden: false, site: NO_SETTINGS, page: seed.page } };
  moved(sessionId);
}

/** The mock's history moved: what shows, and whether Back and Forward go anywhere. */
function moved(sessionId: string): void {
  const history = visits.get(sessionId);
  if (history === undefined) return;
  edit(sessionId, (one) => ({ ...one, url: history.visited[history.at] ?? "", back: history.at > 0, forward: history.at < history.visited.length - 1 }));
}

function walked(sessionId: string, step: (history: { visited: string[]; at: number }) => void): void {
  const history = visits.get(sessionId);
  if (history === undefined) return;
  step(history);
  moved(sessionId);
}

/** Take over, or hand back to the agent. */
export function toggleDriver(sessionId: string): void {
  if (wired !== undefined) void wired.drive(sessionId);
  else edit(sessionId, (one) => ({ ...one, driver: one.driver === "agent" ? "you" : "agent", site: { ...one.site, pending: null } }));
}

/** The host a page is on. */
export const hostOf = (url: string): string => {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
};

export function setChoice(sessionId: string, host: string, permission: SitePermission, choice: Choice): void {
  if (wired !== undefined) {
    void wired.choose(sessionId, host, permission, choice);
    return;
  }
  edit(sessionId, (one) => ({ ...one, site: { ...one.site, sites: { ...one.site.sites, [host]: { ...one.site.sites[host], [permission]: choice } } } }));
}
export function setDefault(sessionId: string, permission: SitePermission, choice: Choice): void {
  if (wired !== undefined) {
    void wired.setDefault(sessionId, permission, choice);
    return;
  }
  edit(sessionId, (one) => ({ ...one, site: { ...one.site, defaults: { ...one.site.defaults, [permission]: choice } } }));
}
/** **The mock's** page asking. While the agent drives it is refused and nothing is kept; a decided site is answered at once; only an undecided one raises the bubble. */
export function requestPermission(sessionId: string, permission: SitePermission): void {
  edit(sessionId, (one) => {
    if (one.driver !== "you") return one;
    const host = hostOf(one.url);
    if (effectiveChoice(one.site, host, permission) !== "ask") return one;
    return { ...one, site: { ...one.site, pending: { permission, host } } };
  });
}
export function answerRequest(sessionId: string, choice: "allow" | "block"): void {
  if (wired !== undefined) {
    void wired.answer(sessionId, choice);
    return;
  }
  edit(sessionId, (one) => {
    const asked = one.site.pending;
    if (asked === null) return one;
    return { ...one, site: { ...one.site, pending: null, sites: { ...one.site.sites, [asked.host]: { ...one.site.sites[asked.host], [asked.permission]: choice } } } };
  });
}
/** Esc on the bubble: the site stays undecided. */
export function dismissRequest(sessionId: string): void {
  if (wired !== undefined) {
    void wired.answer(sessionId, "dismiss");
    return;
  }
  edit(sessionId, (one) => ({ ...one, site: { ...one.site, pending: null } }));
}
/** A new address, which drops whatever Forward would have gone to. */
export function navigate(sessionId: string, url: string): void {
  if (wired !== undefined) void wired.navigate(sessionId, url);
  else
    walked(sessionId, (history) => {
      history.visited.splice(history.at + 1, Infinity, url);
      history.at += 1;
    });
}
export function goBack(sessionId: string): void {
  if (wired !== undefined) void wired.go(sessionId, "back");
  else walked(sessionId, (history) => void (history.at = Math.max(0, history.at - 1)));
}
export function goForward(sessionId: string): void {
  if (wired !== undefined) void wired.go(sessionId, "forward");
  else walked(sessionId, (history) => void (history.at = Math.min(history.visited.length - 1, history.at + 1)));
}
export function reload(sessionId: string): void {
  if (wired !== undefined) void wired.go(sessionId, "reload");
  else edit(sessionId, (one) => ({ ...one, loads: one.loads + 1 }));
}
export const showThread = (sessionId: string, shown: boolean) => edit(sessionId, (one) => ({ ...one, threadHidden: !shown }));
/** The Job browser in Bridge's side panel, out of any window it had. Main makes an empty one where the Job has none. */
export function openBrowser(sessionId: string): void {
  if (wired !== undefined) {
    asked.add(sessionId);
    void wired.show(sessionId);
    return;
  }
  edit(sessionId, (one) => ({ ...one, windowed: false }));
  openInPanel({ kind: "browser", sessionId });
}
/** The Job browser in a window of its own. The side panel it moved from is closed by its own button. */
export function browserToWindow(sessionId: string): void {
  if (wired !== undefined) void wired.toWindow(sessionId);
  else edit(sessionId, (one) => ({ ...one, windowed: true }));
}
/** **The mock's** window close: the page is put away, and the Job keeps it. */
export const closeBrowserWindow = (sessionId: string) => edit(sessionId, (one) => ({ ...one, windowed: false }));
export const pinNote = (sessionId: string, pin: BrowserPin) => edit(sessionId, (one) => ({ ...one, pins: [...one.pins, pin] }));

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
