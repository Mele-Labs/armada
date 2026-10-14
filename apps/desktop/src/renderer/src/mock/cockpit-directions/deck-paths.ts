// The repository's own areas, for the battlefield: a directory tree with a file count for each directory, and the
// directories each Job and Session is changing. **Mock data, invented for the walk.** The real source of what a
// drone touches is Fleet's observed writes for the slot it holds, the same ledger the write-clash detection reads
// (`GET /clashes`); the file counts would be read off the repository. Nothing here reaches Fleet and no wire is
// added. proto/cockpit-flight-deck.

export type Area = { dir: string; kids: readonly (readonly [string, number])[] };

/** Two levels of each repository's directories, by the repository's name, with a file count for each. */
export const TREES: Readonly<Record<string, readonly Area[]>> = {
  armada: [
    { dir: "crates", kids: [["fleet", 150], ["store", 90], ["ipc", 60], ["drone", 55], ["gate", 40], ["config", 25]] },
    { dir: "apps", kids: [["desktop", 230], ["cli", 30]] },
    { dir: "packages", kids: [["components", 160], ["screens", 100], ["protocol", 70], ["jobs", 55], ["tokens", 20]] },
    { dir: "docs", kids: [["concepts", 34], ["contracts", 28], ["practices", 22]] },
  ],
  pocket: [
    { dir: "src", kids: [["ui", 70], ["sw", 25], ["store", 20]] },
    { dir: "public", kids: [] },
    { dir: "tests", kids: [] },
  ],
};

/** Files in a top directory with no children. */
export const BARE: Readonly<Record<string, number>> = { public: 12, tests: 30 };

export type Touch = { path: string; w: number };

/** What each Job and Session is changing, by its title, as paths with how much of the change is there. */
export const TOUCHES: Readonly<Record<string, readonly Touch[]>> = {
  "Cache the manifest read between dispatches": [
    { path: "crates/fleet/src/manifest", w: 5 },
    { path: "crates/config/src", w: 2 },
  ],
  "Add the pause marker column": [
    { path: "crates/ipc/src/pause.rs", w: 3 },
    { path: "packages/protocol/src", w: 3 },
  ],
  "Shorten the reconnect wait": [
    { path: "apps/desktop/src/renderer/src/cockpit", w: 4 },
    { path: "packages/screens/src", w: 1 },
  ],
  // The one overlap: a Session and a Job are both changing the reconnect code in the desktop app.
  "Fix the reconnect wait": [{ path: "apps/desktop/src/renderer/src/cockpit", w: 3 }],
  "Pin the store clock": [{ path: "crates/store/tests", w: 4 }],
  "Gate policy on every run": [{ path: "crates/gate/src", w: 4 }],
  "Migration notes": [{ path: "docs/concepts", w: 2 }],
  "Changelog script": [{ path: "docs/practices", w: 1 }],
  "Retry the claim on a stale lease": [{ path: "crates/drone/src", w: 3 }],
  "Service worker audit": [{ path: "src/sw", w: 3 }],
};

export const touchesOf = (title: string): readonly Touch[] => TOUCHES[title] ?? [];
