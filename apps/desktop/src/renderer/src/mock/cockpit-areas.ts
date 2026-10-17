// The directories the mock draws each repository in, standing in for what Fleet does not report yet: a directory tree to two
// levels with a file count for each (F1 of the flight deck port, #2301). **Invented numbers, for a walk.** Nothing outside the
// mock reads this; the real `AreaSource` is `noAreas`, and a territory then draws its pads and its main flag with no plots.

import type { Area, AreaSource, Kid } from "../cockpit/deck/areas";

/** What Fleet names at most for a directory (`MOST_NAMES` in `crates/fleet/src/territory.rs`). */
const MOST_NAMES = 100;

const STEMS = ["lib", "mod", "types", "client", "server", "state", "events", "routes", "store", "view", "panel", "layout", "format", "parse", "index", "config", "tests", "helpers", "queue", "ledger"];
const EXTENSIONS = ["rs", "ts", "tsx", "md", "toml"];

/** Plausible file names for a directory of `files`, sorted the way git lists them, `src/` ones among them, cut where Fleet cuts. */
const named = (name: string, files: number): Kid => {
  const all = Array.from({ length: files }, (_, at) => {
    const stem = STEMS[at % STEMS.length]!;
    const ext = EXTENSIONS[Math.floor(at / STEMS.length) % EXTENSIONS.length]!;
    const round = Math.floor(at / (STEMS.length * EXTENSIONS.length));
    return `${at % 3 === 0 ? "src/" : ""}${stem}${round === 0 ? "" : `-${round}`}.${ext}`;
  }).sort();
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
