// The order Flight control draws a Job's tiles in, and the keys the Job's tiles and views wear. The two switches, then two runs of buttons; the owner's own tiles come last. Artifacts, Checks,
// Subagents and Bays are not tiles here: they are the Job head's views (`JobViews.tsx`), numbered first. Compact is the context figure's (`ContextFigure.tsx`), and Map is the stage's own switch.

import type { Tile } from "./flight-controls";
import { tileKey, type SessionTileId } from "./session-tiles";

/** The tiles drawn, in order. */
const TILES: readonly SessionTileId[] = ["land", "pause", "judge", "report", "release", "terminal", "browser", "requests"];

/** The layers the Job head draws as views, in order. */
export const VIEW_LAYERS = ["artifacts", "system", "checks", "subagents", "slot"] as const;

/** Each built-in tile in the order given, whatever order the owner's file lists them in. */
const built = (shown: readonly Tile[], ids: readonly string[]): Tile[] => ids.flatMap((id) => shown.filter((one) => one.builtIn !== undefined && one.id === id));

/** The tiles Flight control draws, each in its seat, off or on, so nothing shifts; the owner's own tiles end the row. */
export const flightTilesOf = (shown: readonly Tile[]): Tile[] => [...built(shown, TILES), ...shown.filter((one) => one.builtIn === undefined)];

/** The keys in reading order, ⌥⇧1 to ⌥⇧9 and ⌥⇧0, the ones after the tenth having none. */
const KEY_ORDER: readonly string[] = [...VIEW_LAYERS, "land", "judge", "report", "release", "terminal", "requests"];

/**
 * The key of a built-in tile or view. **It belongs to the place and not to whether the tile is on**, so one going off or on moves no other's.
 * A key the person set on a tile is theirs wherever it stands. Pause keeps its own otherwise, and a custom tile with none has none.
 */
export function flightKey(tile: Tile): string | undefined {
  if (tile.entry?.key !== undefined && !tile.hidden) return tile.key;
  if (tile.builtIn === undefined) return undefined;
  if (tile.id === "pause") return tile.key;
  const at = KEY_ORDER.indexOf(tile.id);
  return at < 0 ? undefined : tileKey(at);
}
