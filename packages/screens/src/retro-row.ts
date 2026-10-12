// The `retro` row of a hosted Session's thread: a retro being written, drawn as it goes
// (`docs/concepts/retro.md`, *A Session's retro*). Fleet appends it on the press and replaces it by
// id as the writing goes, so a thread holds one row per press and a window draws the latest.

import type { PartialRetroItem, RetroRow, RetroStage, SessionRow } from "@armada/protocol";

/**
 * The row as the thread draws it. **The shape of `SessionRetroRowData` in `@armada/components`,
 * spelled here because main reads this file and main has no DOM to read the components with**; the
 * renderer hands one to the other, so a drift is a type error there.
 */
export type RetroRowDrawn = {
  id: string;
  at: string;
  kind: "retro";
  stage: RetroStage;
  stageSince?: number;
  characters: number;
  items: readonly { title?: string; what?: string; fix?: string; who?: "drone" | "agent" | "owner" | "fleet"; landsIn?: "armada" | "kit" | "manifest" }[];
  retro?: number;
  why?: string;
};

const WHO: readonly string[] = ["drone", "agent", "owner", "fleet"];
const LANDS: readonly string[] = ["armada", "kit", "manifest"];

/** Where each stage comes in the writing, so a late frame of an earlier one is told from a later. */
const RANK: Record<RetroStage, number> = { asking_agent: 0, reading_record: 1, writing: 2, written: 3, failed: 3 };

const isRetro = (row: SessionRow): row is RetroRow => row.kind === "retro";

/**
 * Whether `incoming` should replace `held`, two frames of one press's row. **Later wins, and a
 * frame that arrives late never takes the card backwards**: the stage's own instant first, then how
 * far along the stage is, then how much of the answer had arrived. An instant that will not parse
 * is no evidence, and the frame goes through.
 */
export function retroRowIsNewer(held: RetroRow, incoming: RetroRow): boolean {
  const was = Date.parse(held.stage_since);
  const now = Date.parse(incoming.stage_since);
  if (!Number.isNaN(was) && !Number.isNaN(now) && was !== now) return now > was;
  const heldRank: number = RANK[held.stage];
  const incomingRank: number = RANK[incoming.stage];
  if (heldRank !== incomingRank) return incomingRank > heldRank;
  // A retro that ended is not changed by another frame of the same ending.
  if (incomingRank === RANK.written) return held.stage === incoming.stage ? incoming.retro !== held.retro || incoming.why !== held.why : false;
  return incoming.characters >= held.characters && incoming.items.length >= held.items.length;
}

/**
 * A row replaced where its id is held, appended where it is not. **A `retro` row replaces its
 * earlier frame only if it is newer** (`retroRowIsNewer`); every other row is Fleet's last word.
 */
export function withThreadRow<R extends SessionRow>(rows: readonly R[], row: R): R[] {
  const at = rows.findIndex((one) => one.id === row.id);
  if (at < 0) return [...rows, row];
  const held = rows[at]!;
  if (isRetro(held) && isRetro(row) && !retroRowIsNewer(held, row)) return rows as R[];
  return rows.map((one, index) => (index === at ? row : one));
}

/** The item as the card draws it. An end of the route Fleet named something Bridge does not know is left off. */
function itemOf(item: PartialRetroItem): RetroRowDrawn["items"][number] {
  return {
    ...(item.title === undefined ? {} : { title: item.title }),
    ...(item.what === undefined ? {} : { what: item.what }),
    ...(item.fix === undefined ? {} : { fix: item.fix }),
    ...(item.who !== undefined && WHO.includes(item.who) ? { who: item.who as "drone" | "agent" | "owner" | "fleet" } : {}),
    ...(item.lands_in !== undefined && LANDS.includes(item.lands_in) ? { landsIn: item.lands_in as "armada" | "kit" | "manifest" } : {}),
  };
}

/** The row as the thread draws it, with `at` already read as a clock. */
export function retroRowOf(row: RetroRow, at: string): RetroRowDrawn {
  const since = Date.parse(row.stage_since);
  return {
    id: row.id,
    at,
    kind: "retro",
    stage: row.stage,
    ...(Number.isNaN(since) ? {} : { stageSince: since }),
    characters: row.characters,
    items: row.items.map(itemOf),
    ...(row.retro === undefined ? {} : { retro: row.retro }),
    ...(row.why === undefined ? {} : { why: row.why }),
  };
}
