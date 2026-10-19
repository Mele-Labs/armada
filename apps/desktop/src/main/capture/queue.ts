// The one walk window's queue, read off what Bridge already holds — `docs/practices/capture-window.md`.
//
// **A Job's walks are Fleet's waiting items**, `source: "walk"` on the Job's record, one an address it showed and the
// owner has not approved. A pull request's walk has none and is Bridge's own record, kept where it is opened. Both are
// one queue, oldest shown first. Pure, so the rules are read without a window.

import type { SessionRecord } from "@armada/protocol";
import { addressOf, cleanTitle } from "@armada/screens/src/sessions-wire";

import type { CaptureQueueKey } from "../../shared/capture-window";
import { webOrigin } from "./address";
import type { WindowQuestion } from "./window";

/** A Job's walk as its waiting item says it. */
export type StandingWalk = {
  key: string;
  sessionId: string;
  /** The Job's own name, as the Jobs list calls it. */
  job: string;
  /** The window's title. */
  title: string;
  url: string;
  since: string;
};

/** What a Job's walk is keyed by in the window: the Job, and the address its waiting item names. */
export const sessionWalkKey = (sessionId: string, url: string): string => `session:${sessionId} ${url}`;

/** What a pull request's walk is keyed by: neither a Job's nor a server Run's. */
export const reviewWalkKey = (manifestId: string, number: number): string => `review:${manifestId}#${number}`;

/** The Job's own name: its title, or its short address where it has none. */
export const jobName = (record: SessionRecord | undefined, sessionId: string): string => cleanTitle(record?.title) ?? addressOf(sessionId);

/** Every walk standing on a Job's waiting items, oldest shown first. **One entry a walk**: a Job with two is in it twice. */
export function standingWalks(sessions: readonly SessionRecord[]): StandingWalk[] {
  const walks = sessions.flatMap((record) =>
    (record.waiting_for ?? []).flatMap((item): StandingWalk[] => {
      const url = item.act?.kind === "walk" ? item.act.target : undefined;
      if (item.source !== "walk" || url === undefined || webOrigin(url) === null) return [];
      return [{ key: sessionWalkKey(record.id, url), sessionId: record.id, job: jobName(record, record.id), title: item.text, url, since: item.since }];
    }),
  );
  return oldestFirst(walks);
}

/** Oldest first by when each was shown, and in the order given where two say the same moment or one says none. */
export function oldestFirst<T extends { since: string }>(walks: readonly T[]): T[] {
  const at = (one: T): number => {
    const read = Date.parse(one.since);
    return Number.isNaN(read) ? Number.POSITIVE_INFINITY : read;
  };
  return walks.map((one, place) => ({ one, place })).sort((a, b) => at(a.one) - at(b.one) || a.place - b.place).map(({ one }) => one);
}

/** Where a walk goes among `walks`, oldest first: after every one shown at or before it. */
export function placeOf(walks: readonly { since: string }[], since: string): number {
  const shown = Date.parse(since);
  if (Number.isNaN(shown)) return walks.length;
  const after = walks.findIndex((one) => Date.parse(one.since) > shown);
  return after < 0 ? walks.length : after;
}

/**
 * The question a Job's walk shows: its Job's oldest open `window_ask` item on the origin of its page or of any of its takes.
 * **One asked in another walk of the same Job is that walk's**: `askedIn` maps an item to the address the Job asked in, where
 * Bridge heard it asked, and a walk at `url` passes over one asked in another. One Bridge never heard is the first walk's on its origin.
 */
export function questionOf(
  record: SessionRecord | undefined,
  origins: readonly string[],
  bound?: { url: string; askedIn: ReadonlyMap<string, string> },
): WindowQuestion | null {
  for (const item of record?.waiting_for ?? []) {
    if (item.source !== "window_ask" || item.place === undefined) continue;
    if (!origins.includes(webOrigin(item.place.url) ?? "")) continue;
    const window = bound?.askedIn.get(item.id);
    if (bound !== undefined && window !== undefined && window !== bound.url) continue;
    return { id: item.id, question: item.text, options: item.options ?? [], place: item.place };
  }
  return null;
}

/** A key press as Electron reads it, the fields the queue's keys are told apart by. */
export type KeyInput = { type: string; key: string; code: string; meta: boolean; alt: boolean; control: boolean; shift: boolean };

/**
 * Which of the queue's keys a press is: ⌥⌘↓ the walks, ⌥⌘T the takes, ⌥⌘→ and ⌥⌘← the walks after and before, ⌘R Reload and
 * ⌘↵ Approve. Letters go by `code`, which ⌥ rewrites in `key`. `null`: none of them, and the press is the view's own.
 */
export function queueKeyOf(input: KeyInput): CaptureQueueKey | null {
  if (input.type !== "keyDown" || !input.meta || input.control || input.shift) return null;
  if (input.alt) {
    if (input.key === "ArrowDown") return "open";
    if (input.key === "ArrowRight") return "next";
    if (input.key === "ArrowLeft") return "previous";
    return input.code === "KeyT" ? "takes" : null;
  }
  if (input.key === "Enter") return "approve";
  return input.code === "KeyR" ? "reload" : null;
}

/**
 * Whether a field in the page has the cursor, where ⌘↵ is the page's own. Read in a world of its own, as the finder is, so the
 * page's own scripts never see the look; a frame's own field reads as one.
 */
export const FIELD_SOURCE = `(() => {
  const at = document.activeElement;
  return at !== null && (at.isContentEditable === true || /^(INPUT|TEXTAREA|SELECT|IFRAME)$/.test(at.tagName));
})()`;
