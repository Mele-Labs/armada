// Two takes on Flight control while a Job is open, played in the mock on `?take=`: `grouped` draws the Job's tiles in three groups side by side with
// no pages (Job, Tell the agent, Open); `tabs` moves Artifacts, Checks, Subagents and Bays to a tab row at the top of the thread (`JobTabs.tsx`) and
// leaves Flight control one row. Any other value, or none, draws today's pages. A mock comparison of 25 Oct 2026 for the owner's ask that Flight
// control "feels so hard to find anything" in. Only one stays: the one he picks, and this file goes with the other.

import type { Tile } from "./flight-controls";
import type { SessionTileId } from "./session-tiles";

export type FlightTake = "grouped" | "tabs";

/** The take on the page address; Bridge has no query, so it is none there and the pages draw as they did. */
export const flightTake = (): FlightTake | undefined => {
  const take = new URLSearchParams(window.location.search).get("take");
  return take === "grouped" || take === "tabs" ? take : undefined;
};

export type FlightGroup = "job" | "tell" | "open";

export const GROUP_NAME: Record<FlightGroup, string> = { job: "Job", tell: "Tell the agent", open: "Open" };

/** The two that are a state and not a press, drawn as switches. */
export const SWITCHES: readonly string[] = ["land", "pause"];

/** Each group's built-ins in the order drawn. Map is the stage's own switch and is in none. */
const GROUPS: Record<FlightGroup, readonly SessionTileId[]> = {
  job: ["land", "pause", "release"],
  tell: ["ask", "compact", "judge", "report"],
  open: ["artifacts", "checks", "subagents", "slot", "terminal", "requests"],
};

/** The tiles `tabs` leaves in Flight control, after its switches. */
const ROW: readonly SessionTileId[] = ["ask", "compact", "judge", "report", "release", "terminal", "requests"];

/** The layers `tabs` draws as tabs, in order. Thread, which is no layer, comes before them. */
export const TAB_LAYERS = ["artifacts", "checks", "subagents", "slot"] as const;

/** Live ones first and off ones after, each in the order given. */
const liveFirst = (tiles: readonly Tile[], why: (tile: Tile) => string | undefined): Tile[] => [...tiles.filter((one) => why(one) === undefined), ...tiles.filter((one) => why(one) !== undefined)];

const built = (shown: readonly Tile[], ids: readonly string[]): Tile[] => ids.flatMap((id) => shown.filter((one) => one.builtIn !== undefined && one.id === id));

export type Group = { id: FlightGroup; name: string; switches: Tile[]; tiles: Tile[]; cols: number };

/** `grouped`: the tiles shown, in three groups. The owner's own words and command tiles end Tell the agent, his shell and page tiles end Open; off ones end each group. */
export function groupsOf(shown: readonly Tile[], why: (tile: Tile) => string | undefined): Group[] {
  const own = shown.filter((one) => one.builtIn === undefined);
  const extra: Record<FlightGroup, Tile[]> = { job: [], tell: own.filter((one) => one.kind === "words" || one.kind === "command"), open: own.filter((one) => one.kind === "shell" || one.kind === "page") };
  return (Object.keys(GROUPS) as FlightGroup[]).map((id) => {
    const all = [...built(shown, GROUPS[id]), ...extra[id]];
    const switches = all.filter((one) => one.builtIn !== undefined && SWITCHES.includes(one.id));
    const tiles = liveFirst(all.filter((one) => !switches.includes(one)), why);
    // Two columns wide at least; the rows are two, so a group of seven tiles is four wide.
    return { id, name: GROUP_NAME[id], switches, tiles, cols: Math.max(2, Math.ceil(tiles.length / 2)) };
  });
}

/** `tabs`: the switches, then the tiles of the row, the owner's own after them, off ones last. */
export function rowOf(shown: readonly Tile[], why: (tile: Tile) => string | undefined): { switches: Tile[]; tiles: Tile[] } {
  return { switches: built(shown, SWITCHES), tiles: liveFirst([...built(shown, ROW), ...shown.filter((one) => one.builtIn === undefined)], why) };
}
