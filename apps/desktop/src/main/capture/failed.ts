// A walk window that cannot show its page tells whoever opened the walk: the
// Session that showed it, or the Job whose work it serves. The person should
// not be the one to notice and carry the news back.
//
// **Electron's own events, never the page's word.** A load that failed, an
// HTTP status of 400 or more on the page itself, or the page's process gone.

import { onOrigin } from "./address";

/** Chromium's code for a load something else stopped: this window refusing a navigation, or a newer load. */
const ABORTED = -3;

/**
 * **Told once until a page loads.** A person pressing Reload on a server that is down
 * is the same fact again, and the walk's owner heard it the first time.
 */
export class FailedLoads {
  private told = false;
  private readonly tell: (url: string, why: string) => void;

  constructor(tell: (url: string, why: string) => void) {
    this.tell = tell;
  }

  /** `did-fail-load`. A subframe's failure leaves the page walkable. */
  failedLoad(code: number, description: string, url: string, mainFrame: boolean): void {
    if (!mainFrame || code === ABORTED) return;
    this.say(url, description === "" ? `load failed (${code})` : description);
  }

  /**
   * `did-navigate`, the main frame's alone. A page on the window's own origin that answered
   * under 400 is one that loaded, and the next failure is news again. Chromium's own error page
   * is on no origin of the walk's, so it clears nothing.
   */
  navigated(url: string, status: number, origin: string): void {
    if (status >= 400) this.say(url, `HTTP ${status}`);
    else if (status >= 200 && onOrigin(origin, url)) this.told = false;
  }

  /** `render-process-gone`. A clean exit is the window closing, not the page failing. */
  gone(url: string, reason: string): void {
    if (reason === "clean-exit") return;
    this.say(url, `the page's process ended: ${reason}`);
  }

  private say(url: string, why: string): void {
    if (this.told) return;
    this.told = true;
    this.tell(url, why);
  }
}
