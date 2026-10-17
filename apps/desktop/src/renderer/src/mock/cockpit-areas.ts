// The directories the mock draws each repository in, standing in for what Fleet does not report yet: a directory tree to two
// levels with a file count for each (F1 of the flight deck port, #2301). **Invented numbers, for a walk.** Nothing outside the
// mock reads this; the real `AreaSource` is `noAreas`, and a territory then draws its pads and its main flag with no plots.

import type { Area, AreaSource, Kid } from "../cockpit/deck/areas";

/** What Fleet names at most for a directory (`MOST_NAMES` in `crates/fleet/src/territory.rs`). */
const MOST_NAMES = 100;

const STEMS = ["lib", "mod", "types", "client", "server", "state", "events", "routes", "store", "view", "panel", "layout", "format", "parse", "index", "config", "tests", "helpers", "queue", "ledger"];
const EXTENSIONS = ["rs", "ts", "tsx", "md", "toml"];

/** Plausible file names for a directory of `files`, sorted the way git lists them, `src/` ones among them, cut where Fleet cuts. Any `own` are among them. */
const named = (name: string, files: number, own: readonly string[] = []): Kid => {
  const all = [
    ...Array.from({ length: Math.max(0, files - own.length) }, (_, at) => {
      const stem = STEMS[at % STEMS.length]!;
      const ext = EXTENSIONS[Math.floor(at / STEMS.length) % EXTENSIONS.length]!;
      const round = Math.floor(at / (STEMS.length * EXTENSIONS.length));
      return `${at % 3 === 0 ? "src/" : ""}${stem}${round === 0 ? "" : `-${round}`}.${ext}`;
    }).filter((one) => !own.includes(one)),
    ...own,
  ].sort();
  return [name, files, all.slice(0, MOST_NAMES)];
};

const BY_NAME: Readonly<Record<string, readonly Area[]>> = {
  armada: [
    { dir: "crates", kids: [named("fleet", 150), named("store", 90), named("ipc", 60), named("drone", 55), named("gate", 40), named("config", 25)] },
    { dir: "apps", kids: [named("desktop", 230), named("cli", 30)] },
    { dir: "packages", kids: [named("components", 160), named("screens", 100), named("protocol", 70), named("jobs", 55), named("tokens", 20)] },
    { dir: "docs", kids: [named("concepts", 34), named("contracts", 28), named("practices", 22)] },
  ],
  pocket: [
    { dir: "src", kids: [named("ui", 70), named("sw", 25), named("store", 20)] },
    { dir: "public", kids: [], names: ["favicon.ico", "icon-192.png", "icon-512.png", "manifest.webmanifest"] },
    { dir: "tests", kids: [], names: ["fixtures/cart.json", "fixtures/offline.json", "install.spec.ts", "offline.spec.ts", "sync.spec.ts"] },
  ],
};

/** The areas of a repository by the last part of its root, the way the mock's repositories are named. */
export const mockAreas: AreaSource = (root) => BY_NAME[root.split("/").filter((part) => part !== "").at(-1) ?? ""] ?? [];

/**
 * The same repository for the walk `full-rooms`: rooms with a handful of files, a few with dozens and one with hundreds, and the files the
 * `pixi-map` Sessions touch among the names of their rooms, bar one a room names past the names Fleet gives.
 */
const ROOMS: readonly Area[] = [
  {
    dir: "crates",
    kids: [
      named("fleet", 150, ["src/gate.rs"]),
      named("store", 90, ["tests/clock.rs", "tests/pool.rs", "src/clock.rs", "migrations/0043.sql", "migrations/0044.sql"]),
      named("ipc", 60),
      named("drone", 55),
      named("gate", 40, ["src/policy.rs"]),
      named("config", 25),
      named("xtask", 7),
      named("testkit", 12),
    ],
  },
  { dir: "apps", kids: [named("desktop", 640), named("cli", 30)] },
  { dir: "packages", kids: [named("components", 160), named("screens", 100), named("protocol", 70), named("jobs", 55), named("tokens", 5)] },
  { dir: "docs", kids: [named("concepts", 34), named("contracts", 28), named("practices", 22), named("journeys", 5)] },
];

/** The repository of `ROOMS`, by the same rule as `mockAreas`. */
export const mockRooms: AreaSource = (root) => (root.split("/").filter((part) => part !== "").at(-1) === "armada" ? ROOMS : mockAreas(root));
