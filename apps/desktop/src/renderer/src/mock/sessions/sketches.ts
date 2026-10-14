// The sketches a Session's agent draws in the `agent-sketch` walk: three takes on the store clock. The second has the
// first's title, so it replaces it on the ledger, and the third is read against the second. Mock only: `show_sketch`
// hands Fleet the same shape, and `parseScene` checks it where it is drawn.

import { arrow, box, group, label, scene } from "../scenes";

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

// A sequence diagram: each part is a lifeline (a group), and each message an arrow between two points on them, in the
// order it plays. A point is a label, which is only a place for the arrow to meet.
export const SEQUENCE_TITLE = "Session start";

const LIFELINES = ["Bridge", "Fleet", "Drone", "Store"] as const;
const MESSAGES: readonly [from: number, to: number, said: string][] = [
  [0, 1, "start_session"],
  [1, 2, "run"],
  [2, 3, "write"],
  [3, 2, "ok"],
  [2, 1, "done"],
  [1, 0, "session.changed"],
];
const SPACE = 260;
const FIRST = 110;
const GAP = 70;

export const SEQUENCE = scene(
  [
    ...LIFELINES.map((name, at) => group(`life-${name.toLowerCase()}`, at * SPACE, 0, 200, FIRST + MESSAGES.length * GAP, name)),
    ...MESSAGES.flatMap(([from, to], at) => [
      label(`m${String(at + 1)}-from`, from * SPACE + 96, FIRST + at * GAP, "•"),
      label(`m${String(at + 1)}-to`, to * SPACE + 96, FIRST + at * GAP, "•"),
    ]),
  ],
  MESSAGES.map(([, , said], at) => arrow(`m${String(at + 1)}-from`, `m${String(at + 1)}-to`, said, false, at + 1)),
);

// A workflow: nodes in the order they run, a fork after Dispatch and a join at the Gate. An arrow's step lights the
// node it ends at, so the two arrows of a step play together.
export const WORKFLOW_TITLE = "Job gates";

export const WORKFLOW = scene(
  [
    box("dispatch", 0, 140, "Dispatch", "leases a slot", { icon: "rocket", step: 1 }),
    box("build", 330, 20, "Build", "cargo build", { icon: "hammer" }),
    box("test", 330, 260, "Test", "armada check test", { icon: "shield-check" }),
    box("gate", 660, 140, "Gate", "both pass", { icon: "scale" }),
    box("land", 990, 140, "Land", "merge to main", { icon: "git-merge" }),
  ],
  [
    arrow("dispatch", "build", undefined, false, 2),
    arrow("dispatch", "test", undefined, false, 2),
    arrow("build", "gate", undefined, false, 3),
    arrow("test", "gate", undefined, false, 3),
    arrow("gate", "land", undefined, false, 4),
  ],
);
