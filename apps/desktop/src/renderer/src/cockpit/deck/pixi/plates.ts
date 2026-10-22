// What the pointer is over on the ground of the canvas map, found from where it is on the screen: a file's bar, a plot, a base. These are
// hovered and not pressed or tabbed to, and there are as many as the repository has directories and files, so they are not elements: one
// listener asks this on a move, and the answer is the card the CSS map shows for the same plot. Pure.

import { onPlane } from "../bf-parts";
import { FLAT, type Bar, type Tile } from "../layout";
import { FILE_SAID, blameSaid, clashLines, type Tip } from "../tip";
import { pillAt } from "../pills";
import { project, unproject, type View } from "./project";
import type { PixiSceneIn } from "./scene";

/** The thing under the pointer: a key to tell it from the one beside it, the card it says, and the plot it lights, if it is one. */
export type Plate = { key: string; tip: Tip; tile?: string };

type Corner = { x: number; y: number };

/** Whether a point is inside a polygon of the screen. */
function within(polygon: readonly Corner[], x: number, y: number): boolean {
  let inside = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i, i += 1) {
    const [a, b] = [polygon[i]!, polygon[j]!];
    if (a.y > y !== b.y > y && x < ((b.x - a.x) * (y - a.y)) / (b.y - a.y) + a.x) inside = !inside;
  }
  return inside;
}

/** The painter's order the plots are drawn in, so the last to contain a point is the one in front. */
const front = (a: Tile, c: Tile) => a.rect.y + a.rect.h - (c.rect.y + c.rect.h) || a.rect.x + a.rect.w - (c.rect.x + c.rect.w);

const ordered = new WeakMap<object, Tile[]>();
const tilesOf = (scene: PixiSceneIn): Tile[] => {
  let found = ordered.get(scene.layout);
  if (found === undefined) {
    found = scene.layout.territories.flatMap((one) => one.tiles).sort(front);
    ordered.set(scene.layout, found);
  }
  return found;
};

/** A level rectangle of the plane, as the quad it is on the screen. */
const flat = (view: View, x0: number, y0: number, x1: number, y1: number, z: number): Corner[] => [
  project(view, { x: x0, y: y0, z }),
  project(view, { x: x1, y: y0, z }),
  project(view, { x: x1, y: y1, z }),
  project(view, { x: x0, y: y1, z }),
];

/** A slab's top and the two sides the canvas draws, which are the south and the east. */
function slabFaces(view: View, tile: Tile, height: number): Corner[][] {
  const [x0, x1] = [tile.rect.x, tile.rect.x + tile.rect.w];
  const [y0, y1] = [tile.rect.y * FLAT, (tile.rect.y + tile.rect.h) * FLAT];
  const at = (x: number, y: number, z: number) => project(view, { x, y, z });
  return [
    flat(view, x0, y0, x1, y1, height),
    [at(x0, y1, 0), at(x1, y1, 0), at(x1, y1, height), at(x0, y1, height)],
    [at(x1, y0, 0), at(x1, y1, 0), at(x1, y1, height), at(x1, y0, height)],
  ];
}

const barTip = (bar: Bar, blamed: { check: string; line?: number } | undefined): Tip => ({
  label: FILE_SAID[bar.act ?? "rest"],
  title: bar.path,
  hue: blamed === undefined ? "quiet" : "bad",
  ...(blamed === undefined ? {} : { lines: [blameSaid(blamed.check, bar.path, blamed.line)] }),
});

/** What lies under a point of the pane, nearest the viewer first: a file's bar, a plot, a base. */
export function plateAt(scene: PixiSceneIn, view: View, x: number, y: number): Plate | undefined {
  if (view.width === 0) return undefined;
  const tiles = tilesOf(scene);
  const heightOf = (path: string) => scene.heights.get(path) ?? 0;
  for (const bar of scene.layout.bars.values()) {
    const { rect } = bar;
    if (within(flat(view, rect.x, rect.y * FLAT, rect.x + rect.w, (rect.y + rect.h) * FLAT, heightOf(bar.tile.path) + 10), x, y)) return { key: `bar:${bar.path}`, tip: barTip(bar, scene.pad?.blame.find((one) => one.bar === bar.path)) };
  }
  for (let i = tiles.length - 1; i >= 0; i -= 1) {
    const tile = tiles[i]!;
    const faces = slabFaces(view, tile, heightOf(tile.path));
    if (!faces.some((face) => within(face, x, y))) continue;
    // On a plot's top, the pill under the pointer is found from where the pointer is on the plot, not from the pills. One with a name says its file.
    const grid = tile.pills;
    if (grid !== undefined && within(faces[0]!, x, y)) {
      const at = unproject(view, x, y, heightOf(tile.path));
      const i = pillAt(grid, at.x, at.y / FLAT);
      const name = i === undefined ? undefined : grid.names[i];
      if (name !== undefined) return { key: `pill:${tile.path}/${name}`, tip: { label: FILE_SAID.rest, title: `${tile.path}/${name}`, hue: "quiet" } };
    }
    const clash = scene.layout.clashes.find((one) => one.path === tile.path);
    const planned = scene.planned?.get(tile.path);
    return { key: `tile:${tile.path}`, tile: tile.path, tip: clash === undefined ? { label: "Directory", title: tile.path, hue: "quiet", ...(planned === undefined ? {} : { lines: [planned.said] }) } : { label: "Write clash", title: tile.path, hue: "ask", lines: clashLines(clash) } };
  }
  for (const one of scene.layout.territories) {
    const centre = onPlane({ x: one.base.x, y: one.base.y + 4 });
    const half = (one.base.w + 24) / 2;
    if (within(flat(view, centre.x - half, centre.y - 27 * FLAT, centre.x + half, centre.y + 27 * FLAT, 0), x, y)) return { key: `base:${one.root}`, tip: { label: "Repository", title: one.name, hue: "quiet" } };
  }
  return undefined;
}
