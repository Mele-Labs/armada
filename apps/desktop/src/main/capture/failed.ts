// A walk window that cannot show its page tells whoever opened the walk: the
// Session that showed it, or the Job whose work it serves. The person should
// not be the one to notice and carry the news back.
//
// **Electron's own events, and main's own looks, never the page's word.** A
// load that failed, an HTTP status of 400 or more on the page itself, the
// page's process gone, a load still going after `LOAD_MS`, or a loaded page
// that shows Vite's error overlay or draws nothing.

import { onOrigin } from "./address";

/** Chromium's code for a load something else stopped: this window refusing a navigation, or a newer load. */
const ABORTED = -3;

/** Looks in a row a page must read blank before it is told: one is a page still drawing. */
const BLANK_LOOKS = 2;

/** How long a load may go on before it is told: a server that took the connection and never answered. */
export const LOAD_MS = 30_000;

/** What one look at a loaded page found. */
export type Looked = "overlay" | "blank" | "fine";

/**
 * One look at the page, as fixed text main runs in it. **It answers one of three words and never
 * the page's own text**: the page is untrusted, and what it says would reach an agent's turn.
 */
export function lookSource(): string {
  return `(() => {
    if (document.querySelector("vite-error-overlay") !== null) return "overlay";
    const body = document.body;
    if (body === null) return "blank";
    if (body.innerText.trim() !== "") return "fine";
    for (const one of body.querySelectorAll("img, svg, canvas, video, iframe, input, button, textarea, select")) {
      const box = one.getBoundingClientRect();
      if (box.width > 0 && box.height > 0) return "fine";
    }
    return "blank";
  })()`;
}

/** The page's answer, held to the three words. Anything else is no answer. */
export function looked(answer: unknown): Looked | null {
  return answer === "overlay" || answer === "blank" || answer === "fine" ? answer : null;
}

/**
 * **Told once until the page is seen working.** A person pressing Reload on a server that is down,
 * or on a page that still fails to compile, is the same fact again, and the walk's owner heard it
 * the first time. Working is a page on the window's own origin that answered under 400 and then
 * read fine.
 */
export class FailedLoads {
  private told = false;
  /** Whether the page on screen is one that loaded: on the walk's origin, under 400. */
  private loaded = false;
  private blanks = 0;
  private stalled: ReturnType<typeof setTimeout> | null = null;
  private readonly tell: (url: string, why: string) => void;

  constructor(tell: (url: string, why: string) => void) {
    this.tell = tell;
  }

  /** `did-fail-load`. A subframe's failure leaves the page walkable. */
  failedLoad(code: number, description: string, url: string, mainFrame: boolean): void {
    if (!mainFrame || code === ABORTED) return;
    this.loaded = false;
    this.say(url, description === "" ? `load failed (${code})` : description);
  }

  /** `did-navigate`, the main frame's alone. Chromium's own error page is on no origin of the walk's. */
  navigated(url: string, status: number, origin: string): void {
    this.blanks = 0;
    this.loaded = status >= 200 && status < 400 && onOrigin(origin, url);
    if (status >= 400) this.say(url, `HTTP ${status}`);
  }

  /** `render-process-gone`. A clean exit is the window closing, not the page failing. */
  gone(url: string, reason: string): void {
    if (reason === "clean-exit") return;
    this.loaded = false;
    this.say(url, `the page's process ended: ${reason}`);
  }

  /** `did-start-loading`. `where` is read when the time runs out, since a stalled load has committed no address. */
  started(where: () => string): void {
    this.stopped();
    this.stalled = setTimeout(() => {
      this.stalled = null;
      this.say(where(), `the page has not loaded after ${LOAD_MS / 1000} seconds`);
    }, LOAD_MS);
  }

  /** `did-stop-loading`, a load that failed, or the window letting the page go. */
  stopped(): void {
    if (this.stalled !== null) clearTimeout(this.stalled);
    this.stalled = null;
  }

  /** One look at the loaded page. `null`: nothing answered, which says nothing either way. */
  look(url: string, found: Looked | null): void {
    if (found === null) return;
    if (found === "fine") {
      this.blanks = 0;
      if (this.loaded) this.told = false;
    } else if (found === "overlay") {
      this.blanks = 0;
      this.say(url, "the page shows Vite's error overlay");
    } else if ((this.blanks += 1) >= BLANK_LOOKS) {
      this.say(url, "the page loaded with nothing drawn");
    }
  }

  private say(url: string, why: string): void {
    if (this.told) return;
    this.told = true;
    this.tell(url, why);
  }
}
