// The sketches a Session's agent draws in the `agent-sketch` walk: three takes on the store clock. The second has the
// first's title, so it replaces it on the ledger, and the third is read against the second. Mock only: `show_sketch`
// hands Fleet the same shape, and `parseScene` checks it where it is drawn.

import { arrow, box, group, scene } from "../scenes";

const writer = box("writer", 0, 60, "Writer", "put(key, value)", { icon: "hard-drive", step: 1 });
const ledger = box("ledger", 640, 60, "Ledger", "stamps each row", { icon: "scroll-text", step: 3 });
const crate = group("crate", -20, 10, 920, 200, "store crate");

export const READS_TITLE = "Store clock reads";

export const READS = scene(
  [
    crate,
    writer,
    box("clock", 320, 60, "Clock", "now() on every write\nwall time", { icon: "clock", step: 2 }),
    ledger,
    box("test", 320, 280, "store_flaky", "fails when the second turns", { icon: "zap", step: 4 }),
  ],
  [arrow("writer", "clock", "reads"), arrow("clock", "ledger", "stamp"), arrow("test", "clock", "crosses a second")],
);

export const READS_AGAIN = scene(
  [
    crate,
    writer,
    box("clock", 320, 60, "Clock", "now() on every write\nwall time", { icon: "clock", step: 2 }),
    ledger,
    box("test", 320, 280, "store_flaky", "fails 4 of 30 runs", { icon: "zap", step: 4 }),
    box("sleep", 0, 280, "sleep(1s)", "tried in the test", { icon: "hammer" }),
  ],
  [arrow("writer", "clock", "reads"), arrow("clock", "ledger", "stamp"), arrow("test", "clock", "crosses a second"), arrow("sleep", "test", "hides it")],
);

export const PINNED_TITLE = "Pinned clock";

export const PINNED = scene(
  [
    crate,
    writer,
    box("clock", 320, 60, "Clock", "trait, injected at open", { icon: "clock", step: 2 }),
    ledger,
    box("test", 320, 280, "store_flaky", "pins FixedClock", { icon: "zap", step: 4 }),
    box("fixed", 640, 280, "FixedClock", "returns one instant", { icon: "lock", step: 5 }),
  ],
  [arrow("writer", "clock", "reads"), arrow("clock", "ledger", "stamp"), arrow("fixed", "clock", "injected"), arrow("test", "fixed", "uses")],
);
