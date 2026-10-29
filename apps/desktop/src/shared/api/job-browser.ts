// The Job browser: one page per Job that its agent drives and the owner may take over, held in main
// (`main/job-browsers.ts`). A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.
// `docs/practices/protocol/the-job-browser.md` is the wire it answers.

import type { Outcome } from "@armada/protocol";
import type { PageBounds } from "@armada/screens/src/draft/sessions";
import type { CaptureHeld } from "@armada/studios/api";

/** One Job's browser as main holds it. `windowed`: in a window of its own, beside the Job's thread. */
export type JobBrowserState = {
  sessionId: string;
  url: string;
  title: string;
  back: boolean;
  forward: boolean;
  driver: "agent" | "owner";
  windowed: boolean;
};

export type JobBrowserApi = {
  /**
   * **Absent in the mock**, whose store plays the browser itself. The renderer names a Job and never an address it
   * did not type, and the page is a view main places over the rect the pane reports.
   */
  jobBrowser?: {
    list: () => Promise<readonly JobBrowserState[]>;
    onChanged: (listener: (state: JobBrowserState) => void) => () => void;
    /** Main asks this window to show the Job's browser in its side panel. */
    onShow: (listener: (sessionId: string) => void) => () => void;
    /** ⌥⌘C pressed while the page held the keyboard. */
    onCapture: (listener: (sessionId: string) => void) => () => void;
    /** Shows the Job's browser in Bridge's side panel, made empty where it has none, and out of any window it was in. */
    show: (sessionId: string) => Promise<void>;
    place: (sessionId: string, bounds: PageBounds) => Promise<void>;
    unplace: (sessionId: string) => Promise<void>;
    drive: (sessionId: string) => Promise<void>;
    go: (sessionId: string, to: "back" | "forward" | "reload") => Promise<void>;
    navigate: (sessionId: string, url: string) => Promise<void>;
    toWindow: (sessionId: string) => Promise<void>;
    /** The page as it stands, as a PNG, held still under Capture: the view is hidden until `thaw`. */
    freeze: (sessionId: string) => Promise<Uint8Array | null>;
    thaw: (sessionId: string) => Promise<void>;
    hold: (sessionId: string, x: number, y: number) => Promise<CaptureHeld | null>;
    /** The box of the part at a point of the held picture, for the ring that follows the pointer; null where nothing is held or found. */
    aim: (sessionId: string, x: number, y: number) => Promise<{ x: number; y: number; width: number; height: number } | null>;
    /** The held part's note, sent to the Job as the owner's. */
    note: (sessionId: string, said: string) => Promise<Outcome>;
  };
};

export const JOB_BROWSER_CHANNELS = {
  jobBrowserList: "bridge:job-browser-list",
  jobBrowserChanged: "bridge:job-browser-changed",
  jobBrowserShow: "bridge:job-browser-show",
  jobBrowserShown: "bridge:job-browser-shown",
  jobBrowserCapture: "bridge:job-browser-capture",
  jobBrowserPlace: "bridge:job-browser-place",
  jobBrowserUnplace: "bridge:job-browser-unplace",
  jobBrowserDrive: "bridge:job-browser-drive",
  jobBrowserGo: "bridge:job-browser-go",
  jobBrowserNavigate: "bridge:job-browser-navigate",
  jobBrowserToWindow: "bridge:job-browser-to-window",
  jobBrowserFreeze: "bridge:job-browser-freeze",
  jobBrowserAim: "bridge:job-browser-aim",
  jobBrowserThaw: "bridge:job-browser-thaw",
  jobBrowserHold: "bridge:job-browser-hold",
  jobBrowserNote: "bridge:job-browser-note",
} as const;
