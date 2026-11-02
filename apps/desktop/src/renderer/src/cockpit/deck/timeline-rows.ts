// What the Timeline HUD says: the Jobs stuck now, then what each other Job came to since he last closed it. Stuck comes from
// the Jobs the list draws, so a Job asking since last week is still here though its event has left the window, and the two
// can never disagree. A Job only working has no row: it has come to nothing yet.

import { CircleCheck, CircleX, GitMerge, type LucideIcon } from "lucide-react";

import type { Session } from "@armada/screens/src/draft/sessions";

import { keyOf, type FleetEvent } from "../fleet-events";
import { waitingOf } from "../waiting";
import { toneOf } from "./order";
import type { Subject } from "./ship";

export type RowTone = "ask" | "bad" | "stuck" | "ok";

export type HudRow = {
  key: string;
  title: string;
  tone: RowTone;
  Glyph: LucideIcon;
  /** What the glyph says, in its tooltip. */
  said: string;
  /** The fact, bare: the question asked, the check that failed, what merged. Nothing where the glyph says it all. */
  fact?: string;
  /** When: nothing where the window holds no time for it. */
  at?: string;
  /** What a press opens. A row with nothing to open, main or a Job that ended, is not a press. */
  opens?: Subject;
};

const isStuck = (one: Subject): boolean => one.kind === "session" && one.paused !== true && one.tab !== true && (toneOf(one) !== "calm" || one.phase === "held" || one.mark.tone === "bad");

const toneOfStuck = (one: Subject): RowTone => {
  const tone = toneOf(one);
  return tone === "calm" ? "stuck" : tone;
};

/** The Jobs stuck now, in the order the list has them: every repository, whatever the list is narrowed to. */
export function stuckRows(subjects: readonly Subject[], sessions: readonly Session[], events: readonly FleetEvent[]): HudRow[] {
  return subjects.filter(isStuck).map((one) => {
    const session = sessions.find((row) => row.id === one.id);
    const item = session === undefined ? undefined : waitingOf(session)[0];
    const failed = one.pr?.ci === "failed" ? `#${one.pr.number} checks failed` : undefined;
    const fact = item?.text ?? failed;
    const owed = (event: FleetEvent) => event.tone === "ask" || event.tone === "bad";
    // A Job gone quiet owes nothing, and how long it has been quiet is the fact.
    const at = item?.since ?? newestOf(events, one.key, owed)?.at ?? newestOf(events, one.key)?.at;
    return { key: one.key, title: one.title, tone: toneOfStuck(one), Glyph: one.mark.Glyph, said: one.mark.said, opens: one, ...(fact === undefined ? {} : { fact }), ...(at === undefined ? {} : { at }) };
  });
}

function newestOf(events: readonly FleetEvent[], key: string, counts: (event: FleetEvent) => boolean = () => true): FleetEvent | undefined {
  let newest: FleetEvent | undefined;
  for (const one of events) {
    if (keyOf(one.subject) !== key || !counts(one)) continue;
    if (newest === undefined || Date.parse(one.at) > Date.parse(newest.at)) newest = one;
  }
  return newest;
}

/** What counts as a Job having come to something, and how each is drawn. */
const OUTCOMES: Partial<Record<FleetEvent["kind"], { Glyph: LucideIcon; said: string; tone: RowTone }>> = {
  "pr.merged": { Glyph: GitMerge, said: "Merged", tone: "ok" },
  "session.ended": { Glyph: CircleCheck, said: "Finished", tone: "ok" },
  "main.red": { Glyph: CircleX, said: "Main red", tone: "bad" },
  "main.green": { Glyph: CircleCheck, said: "Main green", tone: "ok" },
};

const factOf = (event: FleetEvent, clock: (at: string) => string): string | undefined => {
  if (event.kind === "pr.merged") return event.pull === undefined ? undefined : `#${event.pull.number} merged`;
  if (event.kind === "main.red") return `red ${clock(event.at)}`;
  if (event.kind === "main.green") return `green ${clock(event.at)}`;
  return undefined;
};

/** One row for each Job or main that came to something since `since`, the newest first, leaving out what `stuck` already shows. */
export function movedRows(events: readonly FleetEvent[], since: number, stuck: readonly HudRow[], subjects: readonly Subject[], clock: (at: string) => string): HudRow[] {
  const shown = new Set(stuck.map((one) => one.key));
  const by = new Map<string, FleetEvent[]>();
  for (const event of events) {
    if (OUTCOMES[event.kind] === undefined || Date.parse(event.at) < since) continue;
    const key = event.subject.kind === "main" ? `main:${event.repo}` : keyOf(event.subject);
    if (!shown.has(key)) by.set(key, [...(by.get(key) ?? []), event]);
  }
  const rows = [...by].map(([key, own]): HudRow => {
    const oldest = [...own].sort((a, b) => Date.parse(a.at) - Date.parse(b.at));
    const newest = oldest[oldest.length - 1]!;
    const { Glyph, said, tone } = OUTCOMES[newest.kind]!;
    const facts = oldest.flatMap((one) => factOf(one, clock) ?? []);
    const opens = subjects.find((one) => one.key === key);
    return {
      key,
      // Every repository is listed, so a main says whose.
      title: newest.subject.kind === "main" ? `${newest.repo} main`.trim() : newest.subject.title,
      tone,
      Glyph,
      said,
      at: newest.at,
      ...(facts.length === 0 ? {} : { fact: facts.join(", ") }),
      ...(opens === undefined ? {} : { opens }),
    };
  });
  return rows.sort((a, b) => Date.parse(b.at!) - Date.parse(a.at!));
}
