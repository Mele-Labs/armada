// The Job browser: one page per Browser tab of a Job, held in main
// (`main/job-browsers.ts`). A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.
// `docs/practices/protocol/the-job-browser.md` is the wire it answers.

import type { Outcome } from "@armada/protocol";
import type { PageBounds } from "@armada/screens/src/draft/sessions";
import type { CaptureHeld } from "@armada/studios/api";

import type {
  Answer,
  Choice,
  SiteDefaults,
  SitePermission,
  SiteSettings,
} from "../site-permissions";

/** One Browser tab's page as main holds it. `windowed`: in a window of its own, beside the Job's thread. `tabId` is empty for the page an agent opened before the Job had a tab. */
export type JobBrowserState = {
  sessionId: string;
  tabId: string;
  url: string;
  title: string;
  back: boolean;
  forward: boolean;
  windowed: boolean;
  /** The sentence for the agent's act running on this page now (`shared/browser-act.ts`); absent at rest. */
  acting?: string;
  /** What this repository has decided for each site, and the request the page has waiting. */
  site: SiteSettings;
};

export type JobBrowserApi = {
  /**
   * **Absent in the mock**, whose store plays the browser itself. The renderer names a Job and its tab and never an address it
   * did not type, and the page is a view main places over the rect the pane reports.
   */
  jobBrowser?: {
    list: () => Promise<readonly JobBrowserState[]>;
    onChanged: (listener: (state: JobBrowserState) => void) => () => void;
    /** Main asks this window to open a Browser tab for the Job, since its agent opened a page where it has none. */
    onShow: (listener: (sessionId: string) => void) => () => void;
    /** ⌥⌘C pressed while the page held the keyboard. */
    onCapture: (listener: (sessionId: string, tabId: string) => void) => () => void;
    /** The Job's Browser tabs in pane order, each given a page and any other destroyed. Said as the panes change. */
    tabs: (sessionId: string, tabIds: readonly string[]) => Promise<void>;
    /** Back in its pane: the tab's page is taken out of the window it was in. */
    show: (sessionId: string, tabId: string) => Promise<void>;
    place: (sessionId: string, tabId: string, bounds: PageBounds) => Promise<void>;
    unplace: (sessionId: string, tabId: string) => Promise<void>;
    go: (sessionId: string, tabId: string, to: "back" | "forward" | "reload") => Promise<void>;
    navigate: (sessionId: string, tabId: string, url: string) => Promise<void>;
    toWindow: (sessionId: string, tabId: string) => Promise<void>;
    /** The page as it stands, as a PNG, held still under Capture: the view is hidden until `thaw`. */
    freeze: (sessionId: string, tabId: string) => Promise<Uint8Array | null>;
    thaw: (sessionId: string, tabId: string) => Promise<void>;
    hold: (sessionId: string, tabId: string, x: number, y: number) => Promise<CaptureHeld | null>;
    /** The box of the part at a point of the held picture, for the ring that follows the pointer; null where nothing is held or found. */
    aim: (
      sessionId: string,
      tabId: string,
      x: number,
      y: number,
    ) => Promise<{
      x: number;
      y: number;
      width: number;
      height: number;
    } | null>;
    /** The held part's note, sent to the Job as the owner's. */
    note: (sessionId: string, tabId: string, said: string) => Promise<Outcome>;
    /** A choice for one host, kept for the Job's repository. */
    choose: (
      sessionId: string,
      tabId: string,
      host: string,
      permission: SitePermission,
      choice: Choice,
    ) => Promise<void>;
    /** The defaults for every Job's browser, one set for the whole Mac: for a site with no choice of its own. Named by no Job and no tab. */
    defaults: () => Promise<SiteDefaults>;
    setDefault: (permission: SitePermission, choice: Choice) => Promise<void>;
    /** Told whole after a default is set, from any window. */
    onDefaultsChanged: (listener: (defaults: SiteDefaults) => void) => () => void;
    /** The person's answer to the waiting request. */
    answer: (sessionId: string, tabId: string, answer: Answer) => Promise<void>;
  };
};

export const JOB_BROWSER_CHANNELS = {
  jobBrowserList: "bridge:job-browser-list",
  jobBrowserTabs: "bridge:job-browser-tabs",
  jobBrowserChanged: "bridge:job-browser-changed",
  jobBrowserShow: "bridge:job-browser-show",
  jobBrowserShown: "bridge:job-browser-shown",
  jobBrowserCapture: "bridge:job-browser-capture",
  jobBrowserPlace: "bridge:job-browser-place",
  jobBrowserUnplace: "bridge:job-browser-unplace",
  jobBrowserGo: "bridge:job-browser-go",
  jobBrowserNavigate: "bridge:job-browser-navigate",
  jobBrowserToWindow: "bridge:job-browser-to-window",
  jobBrowserFreeze: "bridge:job-browser-freeze",
  jobBrowserAim: "bridge:job-browser-aim",
  jobBrowserThaw: "bridge:job-browser-thaw",
  jobBrowserHold: "bridge:job-browser-hold",
  jobBrowserNote: "bridge:job-browser-note",
  jobBrowserChoose: "bridge:job-browser-choose",
  jobBrowserDefaults: "bridge:job-browser-defaults",
  jobBrowserSetDefault: "bridge:job-browser-set-default",
  jobBrowserDefaultsChanged: "bridge:job-browser-defaults-changed",
  jobBrowserAnswer: "bridge:job-browser-answer",
} as const;
