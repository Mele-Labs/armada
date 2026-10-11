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
  /**
   * The Studio a Note lands on, decided when the window opened. A `null` name
   * is an untitled one. **`null` whole: a Job's server, walked rather than
   * captured** — nothing is armed, because there is nowhere for a Note to land.
   */
  studio: { id: string; name: string | null } | null;
  /** The Job a note lands on where there is no Studio: the one whose server this walks. */
  job?: { id: string; handle: string };
  /** The Session a note goes to, where it showed this page. **Never folded into `job`**: the two are told apart to the panel. */
  session?: { id: string; title: string };
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
};

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

/** The Session's thread or the Job's view beside the page — `capture-window.md`, *The panel beside the page*. */
export type CapturePanel = { kind: "session" | "job"; open: boolean };

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
