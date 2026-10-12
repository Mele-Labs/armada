// The `retro` row of a hosted Session's thread: a retro being written, drawn as it goes
// (`docs/concepts/retro.md`, *A Session's retro*). Fleet appends it on the press and replaces it by
// id as the writing goes, so a thread holds one row per press and a window draws the latest.
//
// **STAND-IN until `@armada/protocol` exports the row.** `RetroRowWire` is the shape Fleet's Rust
// mirror (`crates/ipc/src/session_retro_row.rs`, 23.77) serves, written here by hand so the
// window can be built against it; when the protocol package carries it, delete this type and read
// it from there. Nothing else in this file is a stand-in.

import type { SessionRow } from "@armada/protocol";
import type { Lands, RetroRowItem, RetroRowStage, SessionRetroRowData, Who } from "@armada/components";

/** One item as it settles. A field is absent until its value is complete. */
export type RetroItemWire = {
  title?: string;
  what?: string;
  fix?: string;
  who?: string;
  lands_in?: string;
};

/** STAND-IN: the `retro` row as Fleet serves it. */
export type RetroRowWire = {
  kind: "retro";
  id: string;
  at: string;
  stage: RetroRowStage;
  stage_since: string;
  characters: number;
  items: RetroItemWire[];
  retro?: number;
  why?: string;
};

/** A row of a hosted Session's thread as a window may receive it: the protocol's, and the `retro` row. */
export type ThreadWireRow = SessionRow | RetroRowWire;

const WHO: readonly string[] = ["drone", "agent", "owner", "fleet"];
const LANDS: readonly string[] = ["armada", "kit", "manifest"];

/** Where each stage comes in the writing, so a late frame of an earlier one is told from a later. */
const RANK: Record<RetroRowStage, number> = { asking_agent: 0, reading_record: 1, writing: 2, written: 3, failed: 3 };

const isRetro = (row: ThreadWireRow): row is RetroRowWire => row.kind === "retro";

/**
 * Whether `incoming` should replace `held`, two frames of one press's row. **Later wins, and a
 * frame that arrives late never takes the card backwards**: the stage's own instant first, then how
 * far along the stage is, then how much of the answer had arrived. An instant that will not parse
 * is no evidence, and the frame goes through.
 */
export function retroRowIsNewer(held: RetroRowWire, incoming: RetroRowWire): boolean {
  const was = Date.parse(held.stage_since);
  const now = Date.parse(incoming.stage_since);
  if (!Number.isNaN(was) && !Number.isNaN(now) && was !== now) return now > was;
  const heldRank = RANK[held.stage];
  const incomingRank = RANK[incoming.stage];
  if (heldRank !== incomingRank) return incomingRank > heldRank;
  // A retro that ended is not changed by another frame of the same ending.
  if (incomingRank === RANK.written) return held.stage === incoming.stage ? incoming.retro !== held.retro || incoming.why !== held.why : false;
  return incoming.characters >= held.characters && incoming.items.length >= held.items.length;
}

/**
 * A row replaced where its id is held, appended where it is not. **A `retro` row replaces its
 * earlier frame only if it is newer** (`retroRowIsNewer`); every other row is Fleet's last word.
 */
export function withThreadRow<R extends ThreadWireRow>(rows: readonly R[], row: R): R[] {
  const at = rows.findIndex((one) => one.id === row.id);
  if (at < 0) return [...rows, row];
  const held = rows[at]!;
  if (isRetro(held) && isRetro(row) && !retroRowIsNewer(held, row)) return rows as R[];
  return rows.map((one, index) => (index === at ? row : one));
}

/** The item as the card draws it. An end of the route Fleet named something Bridge does not know is left off. */
function itemOf(item: RetroItemWire): RetroRowItem {
  return {
    ...(item.title === undefined ? {} : { title: item.title }),
    ...(item.what === undefined ? {} : { what: item.what }),
    ...(item.fix === undefined ? {} : { fix: item.fix }),
    ...(item.who !== undefined && WHO.includes(item.who) ? { who: item.who as Who } : {}),
    ...(item.lands_in !== undefined && LANDS.includes(item.lands_in) ? { landsIn: item.lands_in as Lands } : {}),
  };
}

/** The row as the thread draws it, with `at` already read as a clock. */
export function retroRowOf(row: RetroRowWire, at: string): SessionRetroRowData {
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
