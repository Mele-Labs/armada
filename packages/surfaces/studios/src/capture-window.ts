// The capture window's own shapes — #1294, `docs/practices/capture-window.md`.
//
// **Two documents and one of them is not Armada's.** The window shows another
// repository's running web app in a view with no preload, and Bridge's bar
// above it in a view that has Bridge's preload and Bridge's CSP. Everything
// here crosses between main and that bar, which is Bridge on both ends; what
// crosses between main and the page is `StudioCapture` alone, and main
// re-applies every bound on it.
//
// **Nothing here names the Studio's id to the page.** The bar holds it because
// the bar is Armada's own document; the page is told nothing about a Studio, a
// port or a path.

import type { CaptureServed } from "@armada/protocol";

/** A rectangle in the page's viewport, in CSS pixels. */
export type CaptureRect = { x: number; y: number; width: number; height: number };

/** Which hold refused an address, so the bar can say what was refused rather than that something was. */
export type CaptureRefusedWhat = "navigation" | "redirect" | "frame" | "window" | "download";

/** An address this window would not follow, drawn in full and never followed. */
export type CaptureRefusal = {
  address: string;
  what: CaptureRefusedWhat;
  /** Whether the system browser would take it — `http:` and `https:` only. */
  offerable: boolean;
};

/**
 * What the bar draws. **Pushed by main after every change**, so the bar holds
 * no state of its own about the Run, the address or the Studio.
 */
export type CaptureWindowState = {
  /** The Run, as the Note will record it. */
  served: CaptureServed;
  /** What the bar says of the page: its name and the origin, or no name and the address whole where the window was titled by its address. */
  shown: { name: string | null; address: string };
  /**
   * The Studio a Note lands on, decided when the window opened. A `null` name
   * is an untitled one. **`null` whole: a server, walked rather than
   * captured** — nothing is armed, because there is nowhere for a Note to land.
   */
  studio: { id: string; name: string | null } | null;
  /** The Session a note goes to, where it showed this page. */
  session?: { id: string; title: string };
  /**
   * A pull request's walk, opened from the Reviews tab. **No Session showed this page**, so a note goes to `holder`, the title of the
   * Session of his that holds the pull request, or stays in Armada where none does. There is no Approve: it is not a page anyone asked
   * to have approved.
   */
  review?: { title: string; holder?: string };
  /** The panel beside the page: which kind it holds, and whether it is drawn. Absent: a Studio's window, which has none. */
  panel?: CapturePanel;
  /**
   * Whether the Run is still serving. **Capture is refused from the moment it
   * is not**, and the window loads nothing further.
   */
  serving: boolean;
  /** Whether capture is armed: the page is held still and a press points. */
  armed: boolean;
  /** How many subframe navigations were refused since the window opened. */
  framesRefused: number;
  /** The last address refused, or absent. */
  refused?: CaptureRefusal;
  /** A question the Session that opened this window asked about a place on its page. Absent: none open. */
  ask?: CaptureAsk;
  /** The takes of one change the Session showed, in the order the bar draws them, and the one on screen. Absent: one page. */
  variations?: { names: string[]; at: number };
  /** Whether the page is ready to walk, as its Session says. Absent: nobody has said. */
  readiness?: CaptureReadiness;
  /** The walk the page is playing. Absent: the page is not a walk. **Main bounds it; the page said it.** */
  walk?: CaptureWalk;
  /** Every walk the one walk window holds, and which is on screen. Absent: a Studio's window, which holds one page. */
  queue?: CaptureQueue;
  /**
   * The Job this page's notes are held on holds notes not sent yet, one or several. Approve asks first whether to send them.
   * Absent: none are held, or this window's notes are not held.
   */
  held?: "one" | "several";
};

/** One walk in the walk window, as its entry on the bar names it. */
export type CaptureQueued = {
  /** Main's key for the walk, which the bar only hands back. */
  id: string;
  /** The Job's own name, or a pull request's title after its number. */
  name: string;
  /** A pull request's walk: its number. */
  pull?: number;
  /** The window's own title. */
  title: string;
  /** The page's address in full. */
  address: string;
  /** Who its notes go to. */
  aim?: string;
  readiness?: CaptureReadiness;
  /** What its Job asked about the page, where it asked while the walk was not on screen. */
  asking?: string;
};

/** The walks in the window, oldest shown first, and the one on screen by its place among them. */
export type CaptureQueue = { entries: CaptureQueued[]; at: number };

/** A key main heard while the page had the keyboard, which the bar acts on as if it was pressed there. */
export type CaptureQueueKey = "open" | "takes" | "next" | "previous" | "reload" | "approve";

/** Where a walk on the page stands. `at` counts from 1, up to `of`. Text only: the bar draws it as words. */
export type CaptureWalk = {
  name: string;
  at: number;
  of: number;
  /** The step's caption. */
  said: string;
  autoplay: boolean;
  /** The step's target is ringed, so Next has something to act on. */
  ready: boolean;
  /** Past the last step. */
  done: boolean;
  /**
   * Present where **Bridge plays the steps itself**, a Session's `steps`: the bar draws the ring and the
   * caption over the page, since the page draws nothing. Absent where the page plays its own walk and
   * rings its own target.
   */
  guide?: CaptureGuide;
};

/** The ring and caption of a walk Bridge plays, and whether they are in the way. */
export type CaptureGuide = {
  /** The ringed target in the page's viewport. `null`: none is ringed, because the step is being looked for or is not found. */
  box: CaptureRect | null;
  /** Left off, so the page takes presses again. */
  folded: boolean;
  /** The target the step cannot find, as a person would name it. Absent: found, or not yet looked for. */
  stopped?: string;
  /** The page is on a provider the Session named, or has its popup open: the walk waits for the shown origin. Absent: it is not. */
  signingIn?: boolean;
  /** Autoplay stopped before a step that acts on a page off this machine, and Next is waited on. Absent: it is not. */
  waiting?: true;
};

/** `ready`: done and waiting on the person. `working`: still being changed. */
export type CaptureReadiness = "ready" | "working";

/**
 * A question a Session asked in its walk window — `ask_in_window`, `docs/concepts/session.md`. **The
 * selector stays in main**, as a capture's does: the bar gets the box to outline and the words to draw.
 */
export type CaptureAsk = {
  /** The waiting item it settles. */
  id: string;
  question: string;
  options: { label: string; description?: string }[];
  /** Who asks: the Session's title. */
  from: string;
  /**
   * Drawn over the page with the place outlined, or folded into the bar's second row so the page can
   * be used. Folded, the page takes presses again.
   */
  shown: boolean;
  /** The place, in the page's viewport. Absent while it is being looked for; `null` where nothing on the page matched. */
  box?: CaptureRect | null;
};

/** The Session's thread beside the page — `capture-window.md`, *The panel beside the page*. */
export type CapturePanel = { kind: "session"; open: boolean };

/** An answer from the bar: an option by its index, words of the person's own, or both. */
export type CaptureAnswer = { choice?: number; text?: string };

/** What the bar draws over the element under the pointer while capture is armed. */
export type CaptureAimed = { box: CaptureRect; named: string };

/**
 * The element a note is being written on. **The capture itself stays in main**:
 * the bar gets the box to draw and the one line that names what was hit, and
 * the markup, the styles and the selector never leave the process that bounded
 * them.
 */
export type CaptureHeld = { box: CaptureRect; chain: string };

/** A wheel the bar took while armed, forwarded so the page still scrolls under it. */
export type CaptureWheel = { x: number; y: number; deltaX: number; deltaY: number };
