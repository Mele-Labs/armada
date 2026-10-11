// Every capture window open, one per Run — #1294.
//
// **One per Run, and opening again raises the one that is open.** Two windows
// on one origin would be two things a person has to tell apart by their title.

import tokens from "@armada/tokens/tokens.json";

import type { BridgeState } from "../../shared/bridge";
import type { CaptureOpened, SessionRecord } from "@armada/protocol";
import { isPinned, pinned, webOrigin } from "./address";
import { CaptureWindow, type CaptureBoard, type LandsOn, type WindowQuestion } from "./window";

/**
 * How tall the bar is drawn. **The title row's own composition**, which
 * `TitleBar.css` spells as the same three tokens — read here rather than
 * written as a number, so the bar and Bridge's own title row are one height.
 */
function barHeight(): number {
  const of = (name: string): number => {
    const found = tokens.tokens.find((token) => token.name === name);
    return found === undefined ? 0 : Number.parseInt(found.value, 10);
  };
  return of("--space-8") + of("--space-2") + of("--space-1");
}

/** What a Session's window is keyed by, so it cannot be mistaken for a server Run's id. */
const SESSION_KEY = "session:";

export class CaptureWindows {
  private readonly board: CaptureBoard;
  private readonly open = new Map<string, CaptureWindow>();
  /** The sessions main last published, which each Session's window reads its open question off. */
  private sessions: readonly SessionRecord[] | null = null;

  constructor(board: CaptureBoard) {
    this.board = board;
  }

  /**
   * Open the capture window on one server Run, or raise the one already on it.
   *
   * **The address is resolved off the live holder Fleet published**, never off
   * the string the renderer sent — `servers.ts`'s rule, which loading a link in
   * a window is strictly more than.
   */
  openOn(
    state: BridgeState,
    serverId: string,
    url: string,
    studio: { id: string; name: string | null } | null,
  ): CaptureOpened {
    // A Job's server opens with no Studio, to be walked, and what is pointed at
    // there lands on the Job. Any other server still needs its Studio.
    const server = state.servers.servers.find((one) => one.id === serverId);
    const job = state.jobs.find((one) => one.id === server?.job_id);
    const landsOn: LandsOn | null =
      studio !== null ? { studio } : job === undefined ? null : { job: { id: job.id, handle: job.handle } };
    if (landsOn === null) return { ok: false, why: "no_studio" };
    const standing = this.open.get(serverId);
    if (standing !== undefined && standing.open) {
      standing.raise();
      return { ok: true };
    }
    const pin = pinned(state, serverId, url);
    if (!isPinned(pin)) return pin;
    this.open.set(serverId, new CaptureWindow(pin, landsOn, this.board, barHeight()));
    return { ok: true };
  }

  /**
   * Open the window on a page a Session showed, or raise the one it has open on that address.
   *
   * **One per Session**, keyed beside the server Runs'. A Session that shows another address
   * replaces its window rather than opening a second. **Any `http` or `https` address**, unlike a
   * server's window; the origin it is pinned to is the address's own, since the
   * Session named it and there is no live holder to resolve it against as a server's link has.
   */
  openForSession(sessionId: string, title: string, url: string): CaptureOpened {
    const origin = webOrigin(url);
    if (origin === null) return { ok: false, why: "no_address" };
    const key = `${SESSION_KEY}${sessionId}`;
    const standing = this.open.get(key);
    if (standing !== undefined && standing.open) {
      if (standing.address === url) {
        standing.raise();
        return { ok: true };
      }
      standing.close();
    }
    const pin = { run: key, name: title, origin, url, manifestId: null };
    const window = new CaptureWindow(pin, { session: { id: sessionId, title } }, this.board, barHeight());
    this.open.set(key, window);
    // A window opened by a press on a waiting question shows it as one opened by its arrival does.
    if (this.sessions !== null) window.question(questionOf(this.sessions.find((one) => one.id === sessionId), origin));
    return { ok: true };
  }

  /**
   * A Session asked about a place on its walk: **its window comes forward, and opens where it is
   * closed** or is on another origin. `window` is the address the Session showed, which the window
   * opens on; the question itself, and the page it moves to, are read off what Fleet published.
   */
  askedFor(sessionId: string, title: string, window: string, place: string): CaptureOpened {
    const standing = this.open.get(`${SESSION_KEY}${sessionId}`);
    const here = standing !== undefined && standing.open && webOrigin(standing.address) === webOrigin(place);
    const opened = here ? ({ ok: true } as const) : this.openForSession(sessionId, title, window);
    this.ask();
    return opened;
  }

  /** Every Session window shows its Session's oldest open question on its own origin, or none. */
  private ask(): void {
    if (this.sessions === null) return;
    for (const [key, window] of this.open) {
      if (!key.startsWith(SESSION_KEY) || !window.open) continue;
      const record = this.sessions.find((one) => one.id === key.slice(SESSION_KEY.length));
      window.question(questionOf(record, webOrigin(window.address)));
    }
  }

  /**
   * Fold what main published: a Run that has stopped serving ends capture in
   * its window, and the bar says the run ended.
   */
  changed(state: BridgeState): void {
    if (state.sessions.state === "read") this.sessions = state.sessions.sessions;
    for (const [serverId, window] of this.open) {
      if (!window.open) {
        this.open.delete(serverId);
        continue;
      }
      // A Session's page is no Run, so nothing Fleet publishes about servers ends it.
      if (serverId.startsWith(SESSION_KEY)) continue;
      const server = state.servers.servers.find((one) => one.id === serverId);
      if (server === undefined || server.phase === "exited") window.ended();
    }
    this.ask();
  }

  /** The window an IPC call came from, or `null` where it came from anything else. */
  from(contents: Electron.WebContents): CaptureWindow | null {
    for (const window of this.open.values()) {
      if (window.open && window.owns(contents)) return window;
    }
    return null;
  }

  closeAll(): void {
    for (const window of this.open.values()) window.close();
    this.open.clear();
  }
}

/**
 * The question a Session's window shows: its oldest open `window_ask` item on that window's origin.
 * Pure, so the rule is tested without a window.
 */
export function questionOf(record: SessionRecord | undefined, origin: string | null): WindowQuestion | null {
  for (const item of record?.waiting_for ?? []) {
    if (item.source !== "window_ask" || item.place === undefined) continue;
    if (origin === null || webOrigin(item.place.url) !== origin) continue;
    return { id: item.id, question: item.text, options: item.options ?? [], place: item.place };
  }
  return null;
}
