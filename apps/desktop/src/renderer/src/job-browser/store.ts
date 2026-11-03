// A Job's browsers: a page for each Browser tab of the Job's panes, which the Job's agent and the person both use at any time, drawn in the
// tab's pane, or in a window of its own with the Job's thread beside the page, as the walk window draws it. **Main holds the pages**
// (`main/job-browsers.ts`) and tells this store what each shows; the acts here go to main. The mock has no `jobBrowser` on its bridge, so
// there this store plays the browser itself over a page drawn from fixtures (`mock/scenarios/job-browser.ts`), **one for the Job**
// that each of its Browser tabs draws.

import { useSyncExternalStore, type ReactNode } from "react";

import type { BrowserAct } from "@armada/protocol";

import { describeAct } from "../../../shared/browser-act";
import type { JobBrowserApi, JobBrowserState } from "../../../shared/api/job-browser";
import {
  effectiveChoice,
  NO_SETTINGS,
  type Choice,
  type SitePermission,
  type SiteSettings,
} from "../../../shared/site-permissions";
import { viewingNow } from "../cockpit/viewed";
import { jobOpens, look, peek } from "../job-panes/store";
import { activate, panesOf, type Arrangement } from "../job-panes/tree";

/** A note the person pinned on part of the page, at that part's box within the page. Only the mock draws them. */
export type BrowserPin = {
  id: string;
  url: string;
  on: string;
  note: string;
  box: {
    top: number;
    left: number;
    width: number;
    height: number;
  };
};

export type JobBrowser = {
  url: string;
  title: string;
  back: boolean;
  forward: boolean;
  /** Bumped by Reload, so the mock's page is drawn again. */
  loads: number;
  /** The sentence for the agent act running now (`shared/browser-act.ts`); absent at rest. Main says it, and the mock plays it. */
  acting?: string;
  /** **The mock's**: the tab whose page asked, which `site.pending` waits on. The app's request is an item in the Job's Needs you instead. */
  asked?: string;
  /** In a window of its own, which its pane gives way to. */
  windowed: boolean;
  pins: readonly BrowserPin[];
  site: SiteSettings;
  /** **The mock's**: what is drawn where the page goes. In the app the page is a view main places over the pane. */
  page?: (url: string, sessionId: string, tabId: string) => ReactNode;
};

/** Main's, where the bridge has one: the browser is real. */
type Wired = NonNullable<JobBrowserApi["jobBrowser"]>;
let wired: Wired | undefined;

let held: Readonly<Record<string, JobBrowser>> = {};
/** The mock's history for each Job, which main keeps for the app. */
const visits = new Map<
  string,
  {
    visited: string[];
    at: number;
  }
>();
const listeners = new Set<() => void>();
/** The Browser tabs of each Job in pane order, as main was last told them. */
const reported = new Map<string, readonly string[]>();

/**
 * **The mock's**: the defaults every Job's browser reads, one set for all of them. A Job's own `site.defaults` is not read; `shared` puts these
 * in its place. In the app main keeps the defaults, so none of this is used there.
 */
let sharedDefaults: SiteSettings["defaults"] = NO_SETTINGS.defaults;
const sharedViews = new WeakMap<
  JobBrowser,
  {
    defaults: SiteSettings["defaults"];
    view: JobBrowser;
  }
>();

/** The browser as the mock draws it: its own sites over the one set of defaults. The same object comes back while neither has changed. */
function shared(one: JobBrowser): JobBrowser {
  if (wired !== undefined) return one;
  const was = sharedViews.get(one);
  if (was?.defaults === sharedDefaults) return was.view;
  const view: JobBrowser = {
    ...one,
    site: {
      ...one.site,
      defaults: sharedDefaults,
    },
  };
  sharedViews.set(one, {
    defaults: sharedDefaults,
    view,
  });
  return view;
}

/** What a tab's page is held under. **The mock plays one browser for the Job**, so every one of its tabs is held under the Job. */
const keyOf = (sessionId: string, tabId: string): string =>
  wired === undefined ? sessionId : `${sessionId}/${tabId}`;

function told(): void {
  listeners.forEach((hear) => hear());
}

/** `key` is a tab's (`keyOf`), and in the mock the Job's. */
function edit(key: string, change: (one: JobBrowser) => JobBrowser): void {
  const one = held[key];
  if (one === undefined) return;
  held = {
    ...held,
    [key]: change(one),
  };
  told();
}

/** The address showing. */
export const urlOf = (browser: JobBrowser): string => browser.url;

/** The bridge this document talks to, once: what main holds is read, and each change after it is folded in. */
export function wireJobBrowsers(api: Wired): () => void {
  wired = api;
  const fold = (state: JobBrowserState) => {
    const key = keyOf(state.sessionId, state.tabId);
    const was = held[key];
    held = {
      ...held,
      [key]: {
        url: state.url,
        title: state.title,
        back: state.back,
        forward: state.forward,
        loads: was?.loads ?? 0,
        windowed: state.windowed,
        pins: was?.pins ?? [],
        site: state.site,
        ...(state.acting === undefined
          ? {}
          : {
              acting: state.acting,
            }),
      },
    };
    told();
  };
  void api.list().then((all) => all.forEach(fold));
  const offChanged = api.onChanged(fold);
  // An agent's open where the Job has no Browser tab. A Job he has open gets one; the page of any other waits for the tab he opens.
  const offShow = api.onShow((sessionId) => {
    if (viewingNow(sessionId)) openBrowserTab(sessionId);
  });
  return () => {
    offChanged();
    offShow();
    wired = undefined;
  };
}

/** The tab's browser as it stands now, for a caller that is not drawing it; without a tab, the Job's first. */
export function peekJobBrowser(sessionId: string, tabId?: string): JobBrowser | undefined {
  const one =
    tabId !== undefined
      ? held[keyOf(sessionId, tabId)]
      : (held[sessionId] ??
        (reported.get(sessionId) ?? [])
          .map((one) => held[keyOf(sessionId, one)])
          .find((one) => one !== undefined));
  return one === undefined ? undefined : shared(one);
}

/** The tab's browser as it stands, or nothing where it has none. */
export function useJobBrowser(sessionId: string | undefined, tabId = ""): JobBrowser | undefined {
  return useSyncExternalStore(
    (hear) => {
      listeners.add(hear);
      return () => listeners.delete(hear);
    },
    () => {
      const one = sessionId === undefined ? undefined : held[keyOf(sessionId, tabId)];
      return one === undefined ? undefined : shared(one);
    },
  );
}

/** **The mock's**: the defaults every Job's browser reads, for System › Browser. Where main holds them the app has none here yet, so these are the shipped ones. */
export function useBrowserDefaults(): SiteSettings["defaults"] {
  return useSyncExternalStore(
    (hear) => {
      listeners.add(hear);
      return () => listeners.delete(hear);
    },
    () => (wired === undefined ? sharedDefaults : NO_SETTINGS.defaults),
  );
}

/** **The mock's**: one default for every Job's browser. Main's route takes a tab and a repository, so where it is wired this does nothing until one without them exists. */
export function setBrowserDefault(permission: SitePermission, choice: Choice): void {
  if (wired !== undefined) return;
  sharedDefaults = {
    ...sharedDefaults,
    [permission]: choice,
  };
  told();
}

/** The ids of an arrangement's Browser tabs in pane order, which is the order the Job's page is looked for in. */
export const browserTabIds = (arrangement: Arrangement): string[] =>
  panesOf(arrangement.root).flatMap((pane) =>
    pane.tabs.filter((tab) => tab.kind === "browser").map((tab) => tab.id),
  );

/**
 * Tell main which Browser tabs a Job has, so each has a page and the page of a tab that is gone is destroyed. Said as the panes change,
 * by whatever is drawing them: a tab closed while another is in front draws nothing to say so.
 */
export function reportTabs(sessionId: string, tabIds: readonly string[]): void {
  // **The mock**: the page that asked is gone with its tab, and its request with it.
  if (wired === undefined) {
    const asked = held[sessionId]?.asked;
    if (asked !== undefined && !tabIds.includes(asked)) dismissRequest(sessionId, asked);
    return;
  }
  reported.set(sessionId, tabIds);
  // What main no longer holds is not kept here either.
  const mine = `${sessionId}/`;
  const gone = Object.keys(held).filter(
    (key) => key.startsWith(mine) && !tabIds.includes(key.slice(mine.length)),
  );
  if (gone.length > 0) {
    held = Object.fromEntries(Object.entries(held).filter(([key]) => !gone.includes(key)));
    told();
  }
  void wired.tabs(sessionId, tabIds);
}

/** A Browser tab in the Job's panes: the first it has comes to the front, and a Job with none opens one. */
export function openBrowserTab(sessionId: string): void {
  const first = browserTabIds(peek(sessionId))[0];
  if (first === undefined) jobOpens(sessionId, "browser");
  else look(sessionId, (arrangement) => activate(arrangement, first));
}

/** Main's own calls, where this document has them: the pane reports the page's place and holds it still under Capture. */
export const jobBrowserApi = (): Wired | undefined => wired;

/** **The mock's**: a Job's browser on its first addresses, the last showing. */
export function seedJobBrowser(
  sessionId: string,
  seed: {
    visited: string[];
    page: (url: string, sessionId: string, tabId: string) => ReactNode;
  },
): void {
  visits.set(sessionId, {
    visited: [
      ...seed.visited,
    ],
    at: seed.visited.length - 1,
  });
  held = {
    ...held,
    [sessionId]: {
      url: "",
      title: "",
      back: false,
      forward: false,
      loads: 0,
      windowed: false,
      pins: [],
      site: NO_SETTINGS,
      page: seed.page,
    },
  };
  moved(sessionId);
}

/** The mock's history moved: what shows, and whether Back and Forward go anywhere. */
function moved(sessionId: string): void {
  const history = visits.get(sessionId);
  if (history === undefined) return;
  edit(sessionId, (one) => ({
    ...one,
    url: history.visited[history.at] ?? "",
    back: history.at > 0,
    forward: history.at < history.visited.length - 1,
  }));
}

function walked(
  sessionId: string,
  step: (history: { visited: string[]; at: number }) => void,
): void {
  const history = visits.get(sessionId);
  if (history === undefined) return;
  step(history);
  moved(sessionId);
}

/** **The mock's**: an agent act starts, or with none ends, as the mark in the toolbar tells it. */
export function actInMock(sessionId: string, act?: BrowserAct): void {
  edit(sessionId, ({ acting: _was, ...one }) => ({
    ...one,
    ...(act === undefined
      ? {}
      : {
          acting: describeAct(act),
        }),
  }));
}

/** The host a page is on. */
export const hostOf = (url: string): string => {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
};

export function setChoice(
  sessionId: string,
  tabId: string,
  host: string,
  permission: SitePermission,
  choice: Choice,
): void {
  if (wired !== undefined) {
    void wired.choose(sessionId, tabId, host, permission, choice);
    return;
  }
  edit(keyOf(sessionId, tabId), (one) => ({
    ...one,
    site: {
      ...one.site,
      sites: {
        ...one.site.sites,
        [host]: {
          ...one.site.sites[host],
          [permission]: choice,
        },
      },
    },
  }));
}
export function setDefault(
  sessionId: string,
  tabId: string,
  permission: SitePermission,
  choice: Choice,
): void {
  if (wired !== undefined) {
    void wired.setDefault(sessionId, tabId, permission, choice);
    return;
  }
  setBrowserDefault(permission, choice);
}
/** **The mock's** page asking. A decided site is answered at once; only an undecided one raises the bubble. */
export function requestPermission(
  sessionId: string,
  permission: SitePermission,
  tabId: string,
): void {
  edit(sessionId, (one) => {
    const host = hostOf(one.url);
    if (effectiveChoice(shared(one).site, host, permission) !== "ask") return one;
    return {
      ...one,
      asked: tabId,
      site: {
        ...one.site,
        pending: {
          permission,
          host,
        },
      },
    };
  });
}
export function answerRequest(sessionId: string, tabId: string, choice: "allow" | "block"): void {
  if (wired !== undefined) {
    void wired.answer(sessionId, tabId, choice);
    return;
  }
  edit(keyOf(sessionId, tabId), ({ asked: _tab, ...one }) => {
    const asked = one.site.pending;
    if (asked === null) return one;
    return {
      ...one,
      site: {
        ...one.site,
        pending: null,
        sites: {
          ...one.site.sites,
          [asked.host]: {
            ...one.site.sites[asked.host],
            [asked.permission]: choice,
          },
        },
      },
    };
  });
}
/** The request refused now and nothing kept: the site stays undecided. */
export function dismissRequest(sessionId: string, tabId: string): void {
  if (wired !== undefined) {
    void wired.answer(sessionId, tabId, "dismiss");
    return;
  }
  edit(keyOf(sessionId, tabId), ({ asked: _tab, ...one }) => ({
    ...one,
    site: {
      ...one.site,
      pending: null,
    },
  }));
}
/** A new address, which drops whatever Forward would have gone to. */
export function navigate(sessionId: string, tabId: string, url: string): void {
  if (wired !== undefined) void wired.navigate(sessionId, tabId, url);
  else
    walked(sessionId, (history) => {
      history.visited.splice(history.at + 1, Infinity, url);
      history.at += 1;
    });
}
export function goBack(sessionId: string, tabId: string): void {
  if (wired !== undefined) void wired.go(sessionId, tabId, "back");
  else walked(sessionId, (history) => void (history.at = Math.max(0, history.at - 1)));
}
export function goForward(sessionId: string, tabId: string): void {
  if (wired !== undefined) void wired.go(sessionId, tabId, "forward");
  else
    walked(
      sessionId,
      (history) => void (history.at = Math.min(history.visited.length - 1, history.at + 1)),
    );
}
export function reload(sessionId: string, tabId: string): void {
  if (wired !== undefined) void wired.go(sessionId, tabId, "reload");
  else
    edit(sessionId, (one) => ({
      ...one,
      loads: one.loads + 1,
    }));
}
/** The tab's page back in its pane, out of the window it was in. */
export function browserToPane(sessionId: string, tabId: string): void {
  if (wired !== undefined) void wired.show(sessionId, tabId);
  else
    edit(keyOf(sessionId, tabId), (one) => ({
      ...one,
      windowed: false,
    }));
}
/** The tab's page in a window of its own, which its pane gives way to. */
export function browserToWindow(sessionId: string, tabId: string): void {
  if (wired !== undefined) void wired.toWindow(sessionId, tabId);
  else
    edit(keyOf(sessionId, tabId), (one) => ({
      ...one,
      windowed: true,
    }));
}
/** **The mock's** window close: the page is put away, and the Job keeps it. */
export const closeBrowserWindow = (sessionId: string) =>
  edit(sessionId, (one) => ({
    ...one,
    windowed: false,
  }));
export const pinNote = (sessionId: string, pin: BrowserPin) =>
  edit(sessionId, (one) => ({
    ...one,
    pins: [
      ...one.pins,
      pin,
    ],
  }));

/** **The mock's**: the Jobs whose browser has a window of its own. */
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

/** **The mock's**: what a page asks, with the tab it asked on. */
export type BrowserAsk = {
  tabId: string;
  permission: SitePermission;
  host: string;
};

/** **The mock's**: each change in what a Job's page is asking, `null` once it is settled, dismissed or its tab closed. The app's comes from Fleet as an item in the Job's Needs you. */
export function watchBrowserAsks(
  hear: (sessionId: string, ask: BrowserAsk | null) => void,
): () => void {
  const said = new Map<string, string>();
  const check = () => {
    for (const [sessionId, one] of Object.entries(held)) {
      const ask: BrowserAsk | null =
        one.site.pending === null || one.asked === undefined
          ? null
          : {
              tabId: one.asked,
              permission: one.site.pending.permission,
              host: one.site.pending.host,
            };
      const now = ask === null ? "" : `${ask.tabId} ${ask.permission} ${ask.host}`;
      if ((said.get(sessionId) ?? "") === now) continue;
      said.set(sessionId, now);
      hear(sessionId, ask);
    }
  };
  listeners.add(check);
  return () => void listeners.delete(check);
}
