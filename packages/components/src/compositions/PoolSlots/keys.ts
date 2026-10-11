// Where focus goes back to on Cleanup's grid. The keys that move across it are `tile-grid.ts`'s;
// this only knows the grid's class names, so they stay beside the component that draws them.

/** The control that takes focus for a tile: its name, or the add tile's button. */
export const CURSOR = ".armada-bay__name, .armada-bay__add";

/** Focus the tile at `key`, the one whose panel just closed, so the keyboard carries on from it. */
export function focusTile(root: ParentNode, key: string): void {
  root.querySelector<HTMLElement>(`.armada-bay[data-tile="${CSS.escape(key)}"]`)?.querySelector<HTMLElement>(CURSOR)?.focus();
}
