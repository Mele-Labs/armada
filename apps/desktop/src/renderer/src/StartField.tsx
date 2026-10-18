// The Cockpit's own battlefield beside the start menu, as its hero: an empty field at first, then whatever the menu is on, rising. A repository is
// its directories with a drone writing over one, a new project one empty plot with a drone on its pad, and Claude Code's folders a territory each.
// It stands in the right two thirds and fades to the ground toward the menu. The camera drifts slowly and stands still where reduced motion is
// asked for. It reads the field with the same layout, scene and canvas the Cockpit does, and takes no pointer.

import { useEffect, useMemo, useRef, useState } from "react";

import { ShieldCheck } from "lucide-react";

import { homeOf, marksOf, reachOf, type Cam } from "./cockpit/deck/camera";
import { frameOf } from "./cockpit/deck/frame";
import { FOOT, MARGIN } from "./cockpit/deck/geometry";
import { FLAT, bigOf, layoutOf, type DroneIn, type RepoIn, type Size } from "./cockpit/deck/layout";
import { PixiMap } from "./cockpit/deck/pixi/PixiMap";
import { sceneOf } from "./cockpit/deck/pixi/scene";
import type { Areas } from "./cockpit/deck/areas";
import type { PadIn } from "./cockpit/deck/tip";

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
  if (preview === "folder") return [{ root: "/repository", name: "repository", pads: ["/repository:bay-1", "/repository:bay-2"], goal: true, areas: REPOSITORY }];
  if (preview === "new") return [{ root: "/project", name: "project", pads: ["/project:bay-1"], goal: true, areas: NEW }];
  if (preview === "found") {
    const names = folders.length === 0 ? FALLBACK : folders.slice(0, 4).map((one) => one.split("/").pop() ?? one);
    return names.map((name, at) => ({ root: `/${name}-${at}`, name, pads: [`/${name}-${at}:bay-1`], goal: true, areas: FOLDER }));
  }
  return [];
};

/** The drone over the field: writing in a repository's source, or landed on a new project's pad. */
const droneOf = (preview: Preview): DroneIn[] =>
  preview === "folder" || preview === "new"
    ? [
        {
          key: "session:preview",
          kind: "session",
          title: "Job",
          root: preview === "folder" ? "/repository" : "/project",
          said: "Working",
          hue: "calm",
          spin: preview === "folder",
          hover: preview === "folder",
          ping: false,
          stuck: false,
          flare: false,
          working: preview === "folder",
          shown: true,
          slot: preview === "folder" ? "/repository:bay-1" : "/project:bay-1",
          ...(preview === "folder" ? { doing: "write" as const, touches: [{ path: "src/components", w: 1 }] } : { touches: [] }),
        },
      ]
    : [];

/** A bay's pad as the map draws it. */
const padOf = (key: string): PadIn => ({ key, slot: Number(key.slice(-1)), bay: "held", word: "Held", repo: key.split(":")[0]!.slice(1), Glyph: ShieldCheck });

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
  }, [repos, preview]);

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
    // The field is laid out in part of the pane and the camera is moved to put it in the right two thirds, clear of the menu.
    const world: Size = { width: Math.round(size.width * 0.78), height: Math.round(size.height * 0.9) };
    const drones = droneOf(preview);
    const layout = layoutOf(repos, drones, world, big);
    const frame = frameOf({ layout, drones, plays: [], big });
    const plane = { width: world.width, height: world.height * FLAT };
    // At rest the field fits the part of the pane the menu leaves, fitted to a smaller pane and then moved over.
    const part: Size = { width: size.width * 0.66, height: size.height * 0.92 };
    const home = homeOf(part, plane, marksOf(layout.territories, [...layout.at.values()].map((at) => ({ at, call: false })), big, reachOf(FOOT, MARGIN, big)));
    const pads = new Map(layout.territories.flatMap((one) => one.pads.map((pad) => [pad.key, padOf(pad.key)] as const)));
    const padAt = new Map(layout.territories.flatMap((one) => one.pads.map((pad) => [pad.key, { x: pad.x, y: pad.y * FLAT }] as const)));
    return { big, world, layout, frame, home, drones, pads, padAt, zero: new Map([...frame.heights].map(([path]) => [path, 0] as const)) };
  }, [size, repos, preview]);

  const scene = useMemo(() => {
    if (read === undefined) return undefined;
    const { big, world, layout, frame, drones, pads, padAt, zero } = read;
    return sceneOf({
      layout,
      world,
      big,
      frame: risen ? frame : { ...frame, heights: zero },
      drones,
      pads,
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
      loops: new Set(drones.filter((one) => one.spin).map((one) => one.key)),
      locked: false,
      trails: new Map(),
      padAt,
    });
  }, [read, risen]);

  const cam = useMemo<Cam | undefined>(() => {
    if (read === undefined) return undefined;
    const { home } = read;
    // Moving the camera left puts the field to the right of the middle: by the share of the pane the menu holds. The lower, nearer angle, and a
    // little larger than the fit, so the field is the subject and its edge may leave the pane.
    const aside = (size.width * 0.17) / home.eye;
    return { ...home, tilt: true, x: home.x - aside + drift.x, y: home.y + drift.y, eye: home.eye * 1.1 * (1 + drift.eye) };
  }, [read, drift, size.width]);

  return (
    <div ref={host} className="armada-start-field" aria-hidden="true">
      {scene === undefined || cam === undefined ? null : <PixiMap size={size} cam={cam} live scene={scene} />}
    </div>
  );
}
