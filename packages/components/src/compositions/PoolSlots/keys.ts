// Where the keyboard's cursor goes on Cleanup's grid. **The cursor is DOM focus**: a tile is
// focused through the one control that opens it (its name, or the add tile's button), so a tile
// reached by Tab, by a press or by an arrow is the same tile. What a key means is the keymap's,
// read by the surface; this file only knows the grid's geometry, so the class names stay here.

/** Which way a press moves: spatially on the arrows, in drawn order on `j`/`k`. */
export type TileMove = "left" | "right" | "up" | "down" | "next" | "previous";

const CURSOR = ".armada-bay__name, .armada-bay__add";

/** Every tile's cursor under `root`, in the order the DOM has them, which is the order they are drawn. */
function cursors(root: ParentNode): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(`.armada-pool-slots > .armada-bay`))
    .map((tile) => tile.querySelector<HTMLElement>(CURSOR))
    .filter((one): one is HTMLElement => one !== null);
}

/** The tile `element` is inside, or `null` where it is in none. */
export function tileOf(element: Element | null): HTMLElement | null {
  return element?.closest<HTMLElement>(".armada-pool-slots > .armada-bay") ?? null;
}

/**
 * The cursor to focus after a move from the tile holding `from`: the first tile where focus is on
 * none, clamped at the ends rather than wrapped, and the nearest tile that way on an arrow, which
 * crosses from one pool's grid into the next because it reads where tiles are drawn.
 */
export function tileAfter(root: ParentNode, from: Element | null, move: TileMove): HTMLElement | undefined {
  const all = cursors(root);
  const tile = tileOf(from);
  const at = tile === null ? -1 : all.findIndex((one) => tileOf(one) === tile);
  if (at < 0) return all[0];
  if (move === "next") return all[Math.min(at + 1, all.length - 1)];
  if (move === "previous") return all[Math.max(at - 1, 0)];
  const here = tile!.getBoundingClientRect();
  const mid = (rect: DOMRect) => ({ x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 });
  const me = mid(here);
  let best: { cursor: HTMLElement; cost: number } | undefined;
  for (const cursor of all) {
    const rect = tileOf(cursor)!.getBoundingClientRect();
    const them = mid(rect);
    // Only tiles wholly that way: a row's neighbours share its band, a column's lie past its edge.
    const ahead =
      move === "right" ? rect.left >= here.right && rect.top < here.bottom && rect.bottom > here.top
      : move === "left" ? rect.right <= here.left && rect.top < here.bottom && rect.bottom > here.top
      : move === "down" ? rect.top >= here.bottom
      : rect.bottom <= here.top;
    if (!ahead) continue;
    const along = move === "left" || move === "right" ? Math.abs(them.x - me.x) : Math.abs(them.y - me.y);
    const across = move === "left" || move === "right" ? Math.abs(them.y - me.y) : Math.abs(them.x - me.x);
    // The next row or column first, then the tile most nearly in line within it.
    const cost = along * 1000 + across;
    if (best === undefined || cost < best.cost) best = { cursor, cost };
  }
  return best?.cursor;
}

/** Open the panel of the tile holding `from`, as a press on its name does. `false` where nothing opened. */
export function openTile(from: Element | null): boolean {
  const name = tileOf(from)?.querySelector<HTMLElement>(".armada-bay__name");
  if (name === null || name === undefined) return false;
  name.click();
  return true;
}

/** Focus the tile at `key`, the one whose panel just closed, so the keyboard carries on from it. */
export function focusTile(root: ParentNode, key: string): void {
  root.querySelector<HTMLElement>(`.armada-bay[data-tile="${CSS.escape(key)}"]`)?.querySelector<HTMLElement>(CURSOR)?.focus();
}
