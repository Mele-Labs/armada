// The `retro` row of a hosted Session's thread: a retro being written, drawn as it goes
// (`docs/concepts/retro.md`, *A Session's retro*). Fleet appends it on the press and replaces it by
// id as the writing goes, so a thread holds one row per press and a window draws the latest.

import type { RetroStage, SessionRow, PartialRetroItem } from "../../wire/index";

type RetroRow = Extract<
  SessionRow,
  {
    kind: "retro";
  }
>;

/**
 * The row as the thread draws it. **The shape of `SessionRetroRowData` in
 * `kit/components/compositions/SessionRetroRow`**; a drift is a type error where one is handed to the other.
 */
export type RetroRowDrawn = {
  id: string;
  at: string;
  kind: "retro";
  stage: RetroStage;
  stageSince?: number;
  characters: number;
  items: readonly {
    title?: string;
    what?: string;
    fix?: string;
    who?: "drone" | "agent" | "owner" | "fleet";
    landsIn?: "armada" | "kit" | "manifest";
  }[];
  retro?: number;
  why?: string;
};

const WHO: readonly string[] = [
  "drone",
  "agent",
  "owner",
  "fleet",
];
const LANDS: readonly string[] = [
  "armada",
  "kit",
  "manifest",
];

/** The item as the card draws it. An end of the route Fleet named something Bridge does not know is left off. */
function itemOf(item: PartialRetroItem): RetroRowDrawn["items"][number] {
  return {
    ...(item.title === undefined
      ? {}
      : {
          title: item.title,
        }),
    ...(item.what === undefined
      ? {}
      : {
          what: item.what,
        }),
    ...(item.fix === undefined
      ? {}
      : {
          fix: item.fix,
        }),
    ...(item.who !== undefined && WHO.includes(item.who)
      ? {
          who: item.who as "drone" | "agent" | "owner" | "fleet",
        }
      : {}),
    ...(item.lands_in !== undefined && LANDS.includes(item.lands_in)
      ? {
          landsIn: item.lands_in as "armada" | "kit" | "manifest",
        }
      : {}),
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
    ...(Number.isNaN(since)
      ? {}
      : {
          stageSince: since,
        }),
    characters: row.characters,
    items: row.items.map(itemOf),
    ...(row.retro === undefined
      ? {}
      : {
          retro: row.retro,
        }),
    ...(row.why === undefined
      ? {}
      : {
          why: row.why,
        }),
  };
}
