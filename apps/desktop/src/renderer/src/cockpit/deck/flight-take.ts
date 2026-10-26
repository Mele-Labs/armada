// Two takes on Flight control while a Job is open, played in the mock on `?take=`: `grouped` draws the Job's tiles in three groups side by side with
// no pages (Job, Tell the agent, Open); `tabs` moves Artifacts, Checks, Subagents and Bays to tabs on the Job's head (`JobTabs.tsx`, in the look `&tabs=nav`, the
// default, under the title or with `&tabpos=foot` under the composer, or `&tabs=segmented`) and leaves Flight control one row. In both takes Compact is the context figure's (`ContextFigure.tsx`) and the merge
// line is the bottom of the Cockpit's centre, under Flight control. Any other value, or none, draws today's pages. A mock comparison of 25 Oct 2026 for the
// owner's ask that Flight control "feels so hard to find anything" in. Only one stays: the one he picks, and this file goes with the other.

import type { Tile } from "./flight-controls";
import { tileKey, type SessionTileId } from "./session-tiles";

export type FlightTake = "grouped" | "tabs";

/** The take on the page address; Bridge has no query, so it is none there and the pages draw as they did. */
export const flightTake = (): FlightTake | undefined => {
  const take = new URLSearchParams(window.location.search).get("take");
  return take === "grouped" || take === "tabs" ? take : undefined;
};

export type TabsLook = "nav" | "segmented";

/** The look of the tabs on the page address, `nav` where none is given. */
export const tabsLook = (): TabsLook => (new URLSearchParams(window.location.search).get("tabs") === "segmented" ? "segmented" : "nav");

export type TabsPos = "head" | "foot";

/** Where the `nav` tabs stand on the page address: `foot` is the Job panel's last row, under the composer; otherwise under the title. The `segmented` look is in the title row either way. */
export const tabsPos = (): TabsPos => (tabsLook() === "nav" && new URLSearchParams(window.location.search).get("tabpos") === "foot" ? "foot" : "head");

export type Density = "tight" | "tight-type";

/**
 * A mock take of 26 Oct 2026 on `&density=`, for the owner's ask that the Cockpit "is just not utilizing space very well": `tight` is the chrome alone (gutters, header bands, rows, controls),
 * `tight-type` is that and the type ladder one step smaller inside the Cockpit (`density-take.css`). Any other value, or none, draws today's.
 */
export const densityTake = (): Density | undefined => {
  const density = new URLSearchParams(window.location.search).get("density");
  return density === "tight" || density === "tight-type" ? density : undefined;
};

export type ViewsAt = "composer" | "head" | "left";

/**
 * Where the `tabs` take puts the Job's four views on `&views=`, and no tab row in any: `composer` is icon buttons in the message box's row, `head` icon buttons in the Job's title row,
 * `left` a panel under the Jobs list (`JobViews.tsx`). None, or any other value, is the tab row.
 */
export const viewsAt = (): ViewsAt | undefined => {
  const at = new URLSearchParams(window.location.search).get("views");
  return flightTake() === "tabs" && (at === "composer" || at === "head" || at === "left") ? at : undefined;
};

export const viewsInComposer = (): boolean => viewsAt() === "composer";

export type FlightGroup = "job" | "tell" | "open";

export const GROUP_NAME: Record<FlightGroup, string> = { job: "Job", tell: "Tell the agent", open: "Open" };

/** The two that are a state and not a press, drawn as switches in both takes. */
export const SWITCHES: readonly string[] = ["land", "pause"];

/** Each group's built-ins in the order drawn. Map is the stage's own switch and is in none. */
const GROUPS: Record<FlightGroup, readonly SessionTileId[]> = {
  job: ["land", "pause", "release"],
  tell: ["ask", "judge", "report"],
  open: ["artifacts", "checks", "subagents", "slot", "terminal", "requests"],
};

/** The runs `tabs` draws in Flight control: the two switches, then two runs of buttons with a rule between; the owner's own tiles join the last run. Update is A's alone. */
const RUNS: readonly (readonly SessionTileId[])[] = [["land", "pause"], ["judge", "report"], ["release", "terminal", "requests"]];

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

/** `tabs`: the switches and the two runs of buttons, each in its own order and every one in its seat, off or on, so nothing shifts; the owner's own tiles end the last run. */
export function runsOf(shown: readonly Tile[]): { switches: Tile[]; runs: Tile[][] } {
  const [switches = [], ...runs] = RUNS.map((ids) => built(shown, ids));
  const own = shown.filter((one) => one.builtIn === undefined);
  return { switches, runs: [...runs.slice(0, -1), [...(runs.at(-1) ?? []), ...own]].filter((run) => run.length > 0) };
}

/** Each take numbers its keys in reading order, ⌥⇧1 to ⌥⇧9 and ⌥⇧0, the ones after the tenth having none. */
const KEY_ORDER: Record<FlightTake, readonly string[]> = {
  grouped: ["land", "release", "ask", "judge", "report", "artifacts", "checks", "subagents", "slot", "terminal"],
  tabs: ["artifacts", "checks", "subagents", "slot", "land", "judge", "report", "release", "terminal", "requests"],
};

/**
 * The key of a tile while a take is on. **It belongs to the tile's place and not to whether the tile is on**, so one going off or on moves no other's.
 * Pause keeps its own, and a tile of the owner's has none.
 */
export function takeKey(take: FlightTake, tile: Tile): string | undefined {
  if (tile.builtIn === undefined) return undefined;
  if (tile.id === "pause") return tile.key;
  const at = KEY_ORDER[take].indexOf(tile.id);
  return at < 0 ? undefined : tileKey(at);
}
