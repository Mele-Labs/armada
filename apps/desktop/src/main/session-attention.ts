// Telling the owner a Job has started waiting on him, when he is not looking at Bridge.
//
// **A telling is an item entering a live Session's `waiting_for`, once.** What was waiting when Bridge
// connected is where things stand and is not news, so the first reading only seeds. An item re-posted
// under the same id is the same item. The effects arrive as functions, like every seam in `src/main`,
// so this runs in node with no Electron in it.
//
// **The quiet window is not extended.** Entries inside it go out as one banner; restarting it on each
// arrival would hold the banner back for as long as a trickle lasted. `soundOf` in `telling.ts` is the sound.

import type { SessionRecord, WaitingItem } from "@armada/protocol";

import type { Tone } from "./telling";

/** How long entries are collected before one banner goes out. */
export const QUIET_MS = 2000;

/** The sources that hold a turn on the owner. `waiting.ts` in the renderer holds the same split for a call. */
const BLOCKED = new Set(["ask_card", "permission", "window_ask", "plan", "slot_choice", "delivery"]);

export type Told = { sessionId: string; title: string; body: string; tone: Tone };

export type Effects = {
  /** Post one banner, and land on `sessionId` when it is pressed. */
  show: (told: Told) => void;
  /** Say how many Sessions wait on him, or none. Told on every reading, so the dock follows a change of nothing else. */
  count: (waiting: number) => void;
};

type Entered = { sessionId: string; name: string; item: WaitingItem };

export class SessionAttention {
  private readonly effects: Effects;
  /** `${sessionId}:${itemId}` for every item waiting as of the last reading. */
  private waiting: Set<string> = new Set();
  private seeded = false;
  /** The last count told to the dock, so an unchanged one is not told again. */
  private counted = -1;
  private collecting = new Map<string, Entered>();
  private window: ReturnType<typeof setTimeout> | null = null;

  constructor(effects: Effects) {
    this.effects = effects;
  }

  /**
   * Every Session Bridge holds, and when the list was current. **`readAt` is `null` until Fleet has answered
   * once and the reading is refused**, not seeded: a list built from one `session.changed` before the first
   * read holds a single Session, and seeding off it would tell every other waiting item as new.
   */
  read(sessions: readonly SessionRecord[], readAt: number | null): void {
    if (readAt === null) return;
    const now = new Set<string>();
    const entered: Entered[] = [];
    let sessionsWaiting = 0;
    for (const session of sessions) {
      if (session.state !== "live") continue;
      const items = session.waiting_for ?? [];
      if (items.length > 0) sessionsWaiting += 1;
      for (const item of items) {
        const key = `${session.id}:${item.id}`;
        now.add(key);
        if (this.seeded && !this.waiting.has(key)) {
          entered.push({ sessionId: session.id, name: session.title ?? "Job", item });
        }
      }
    }
    this.waiting = now;
    this.seeded = true;
    if (sessionsWaiting !== this.counted) {
      this.counted = sessionsWaiting;
      this.effects.count(sessionsWaiting);
    }
    if (entered.length === 0) return;
    for (const one of entered) this.collecting.set(`${one.sessionId}:${one.item.id}`, one);
    this.open();
  }

  /** Drop what is collected and never told. Called when the connection stops. */
  close(): void {
    if (this.window !== null) clearTimeout(this.window);
    this.window = null;
    this.collecting.clear();
  }

  private open(): void {
    if (this.window !== null) return;
    this.window = setTimeout(() => {
      this.window = null;
      const batch = [...this.collecting.values()].sort(oldest);
      this.collecting.clear();
      const first = batch[0];
      if (first === undefined) return;
      const others = batch.length - 1;
      this.effects.show({
        sessionId: first.sessionId,
        title: first.name,
        body: others === 0 ? first.item.text : `${first.item.text}\nand ${others} more`,
        tone: batch.some((one) => BLOCKED.has(one.item.source)) ? "blocked" : "waiting",
      });
    }, QUIET_MS);
  }
}

const oldest = (a: Entered, b: Entered): number => Date.parse(a.item.since) - Date.parse(b.item.since) || a.item.id.localeCompare(b.item.id);
