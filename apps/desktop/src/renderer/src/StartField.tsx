// The Cockpit's own battlefield behind the start menu: an empty field at first, then the plots of whatever the menu is on, rising. A repository
// is its directories, a new project one empty plot, and Claude Code's folders a territory each. The camera drifts slowly and stands still where
// reduced motion is asked for. It reads the field with the same layout, scene and canvas the Cockpit does, and takes no pointer.

import { useEffect, useMemo, useRef, useState } from "react";

import { homeOf, marksOf, reachOf, type Cam } from "./cockpit/deck/camera";
import { frameOf } from "./cockpit/deck/frame";
import { FOOT, MARGIN } from "./cockpit/deck/geometry";
import { FLAT, SLAB, bigOf, layoutOf, type RepoIn, type Size } from "./cockpit/deck/layout";
import { PixiMap } from "./cockpit/deck/pixi/PixiMap";
import { sceneOf } from "./cockpit/deck/pixi/scene";
import type { Areas } from "./cockpit/deck/areas";

/** What the field shows: nothing, a repository, one new plot, or the folders Claude Code kept. */
export type Preview = "none" | "folder" | "new" | "found";

/** The directories of a repository, as the Cockpit would read them: a stand-in for the one the person will choose. */
const REPOSITORY: Areas = [
  { dir: "src", kids: [["app", 6], ["components", 9], ["lib", 4]] },
  { dir: "tests", kids: [], files: 5 },
  { dir: "docs", kids: [], files: 3 },
];

/** A folder Claude Code kept sessions for, with the directories drawn for it. */
const FOLDER: Areas = [
  { dir: "src", kids: [], files: 7 },
  { dir: "tests", kids: [], files: 3 },
];

const NEW: Areas = [{ dir: "src", kids: [], files: 1 }];

const FALLBACK = ["api", "site", "app", "scratch"];

const reposOf = (preview: Preview, folders: readonly string[]): RepoIn[] => {
  if (preview === "folder") return [{ root: "/repository", name: "repository", pads: ["bay-1", "bay-2"], goal: true, areas: REPOSITORY }];
  if (preview === "new") return [{ root: "/project", name: "project", pads: ["bay-1"], goal: true, areas: NEW }];
  if (preview === "found") {
    const names = folders.length === 0 ? FALLBACK : folders.slice(0, 4).map((one) => one.split("/").pop() ?? one);
    return names.map((name, at) => ({ root: `/${name}-${at}`, name, pads: ["bay-1"], goal: true, areas: FOLDER }));
  }
  return [];
};

/** The camera's drift: how far it wanders on the plane, and how long a lap takes. */
const DRIFT = { x: 26, y: 14, eye: 0.012, lapMs: 40_000 };

export function StartField({ preview, folders }: { preview: Preview; folders: readonly string[] }) {
  const host = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState<Size>({ width: 0, height: 0 });
  // The plots stand at nothing for a moment, then rise to their height.
  const [risen, setRisen] = useState(false);
  const [drift, setDrift] = useState({ x: 0, y: 0, eye: 0 });

  useEffect(() => {
    const one = host.current;
    if (one === null) return;
    const read = () => setSize((was) => (Math.round(one.clientWidth) === was.width && Math.round(one.clientHeight) === was.height ? was : { width: Math.round(one.clientWidth), height: Math.round(one.clientHeight) }));
    read();
    const watch = new ResizeObserver(read);
    watch.observe(one);
    return () => watch.disconnect();
  }, []);

  const repos = useMemo(() => reposOf(preview, folders), [preview, folders]);
  useEffect(() => {
    setRisen(false);
    if (repos.length === 0) return;
    const wait = window.setTimeout(() => setRisen(true), 80);
    return () => window.clearTimeout(wait);
  }, [repos]);

  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const start = performance.now();
    let last = 0;
    let frame = 0;
    const step = (now: number) => {
      frame = requestAnimationFrame(step);
      // About twenty frames a second is smooth for a drift this slow.
      if (now - last < 48) return;
      last = now;
      const turn = ((now - start) / DRIFT.lapMs) * 2 * Math.PI;
      setDrift({ x: Math.sin(turn) * DRIFT.x, y: Math.cos(turn * 0.7) * DRIFT.y, eye: Math.sin(turn * 0.5) * DRIFT.eye });
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, []);

  const read = useMemo(() => {
    if (size.width === 0 || size.height === 0) return undefined;
    const big = bigOf(size);
    // The field is laid out in part of the pane and the camera is moved to put it in the right-hand part, clear of the menu.
    const world: Size = { width: Math.round(size.width * 0.78), height: Math.round(size.height * 0.9) };
    const layout = layoutOf(repos, [], world, big);
    const frame = frameOf({ layout, drones: [], plays: [], big });
    const plane = { width: world.width, height: world.height * FLAT };
    // At rest the field fits the right-hand part of the pane, clear of the menu: fitted to a smaller pane, then moved over.
    const part: Size = { width: size.width * 0.56, height: size.height * 0.84 };
    const home = homeOf(part, plane, marksOf(layout.territories, [], big, reachOf(FOOT, MARGIN, big)));
    const heights = new Map<string, number>();
    const tiles = layout.territories.flatMap((one) => one.tiles);
    const widest = Math.max(1, ...tiles.map((one) => one.rect.w * one.rect.h));
    for (const tile of tiles) heights.set(tile.path, SLAB.low + (SLAB.high - SLAB.low) * 0.7 * Math.sqrt((tile.rect.w * tile.rect.h) / widest));
    return { big, world, layout, frame, home, heights, zero: new Map(tiles.map((one) => [one.path, 0] as const)) };
  }, [size, repos]);

  const scene = useMemo(() => {
    if (read === undefined) return undefined;
    const { big, world, layout, frame, heights, zero } = read;
    return sceneOf({
      layout,
      world,
      big,
      frame: { ...frame, heights: risen ? heights : zero },
      drones: [],
      pads: new Map(),
      goals: new Map(layout.territories.map((one) => [one.root, { root: one.root, said: "Main", red: false }] as const)),
      prs: [],
      plays: [],
      readable: true,
      builds: [],
      selected: undefined,
      hot: undefined,
      hotTile: undefined,
      hotPart: undefined,
      names: layout.territories.flatMap((one) => one.tiles.filter((tile) => tile.rect.w > 70).map((tile) => ({ tile, text: tile.path.split("/").pop() ?? tile.path, clash: false }))),
      focus: undefined,
      labels: new Set(),
      clashed: new Map(),
      calls: new Map(),
      callOf: () => undefined,
      loops: new Set(),
      locked: false,
      trails: new Map(),
      padAt: new Map(),
    });
  }, [read, risen]);

  const cam = useMemo<Cam | undefined>(() => {
    if (read === undefined) return undefined;
    const { home } = read;
    // Moving the camera left puts the field to the right of the middle: by the share of the pane the menu holds.
    const aside = (size.width * 0.24) / home.eye;
    return { ...home, x: home.x - aside + drift.x, y: home.y + drift.y, eye: home.eye * (1 + drift.eye) };
  }, [read, drift, size.width]);

  return (
    <div ref={host} className="armada-start-field" aria-hidden="true">
      {scene === undefined || cam === undefined ? null : <PixiMap size={size} cam={cam} live scene={scene} />}
    </div>
  );
}
