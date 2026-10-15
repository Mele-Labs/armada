// The directories the mock draws each repository in, standing in for what Fleet does not report yet: a directory tree to two
// levels with a file count for each (F1 of the flight deck port, #2301). **Invented numbers, for a walk.** Nothing outside the
// mock reads this; the real `AreaSource` is `noAreas`, and a territory then draws its pads and its main flag with no plots.

import type { Area, AreaSource } from "../cockpit/deck/areas";

const BY_NAME: Readonly<Record<string, readonly Area[]>> = {
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

/** The areas of a repository by the last part of its root, the way the mock's repositories are named. */
export const mockAreas: AreaSource = (root) => BY_NAME[root.split("/").filter((part) => part !== "").at(-1) ?? ""] ?? [];
