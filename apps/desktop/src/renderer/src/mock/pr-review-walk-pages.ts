// What the Walk tool would write for three of the review mock's pull requests, and the places Armada already remembers.
// **One of each way a place is found**: #1907 has a preview link a bot posted, so its walk is a web walk against that
// address; #1899 touches Fleet, whose place Armada remembers, so its walk is a run of requests against the API there;
// #1903 has neither and asks. The steps are written by hand, as the pull requests' hunks are, and name their targets
// the way a `show_window` step does: by role and name.

import type { WalkStep } from "@armada/protocol";

import type { ApiStep, WalkPlan, WalkSeed } from "../pr-review/walk-model";

const KEY = "mele-labs/armada";

// **The steps are `show_window` steps** (`WalkStep`). In the mock the page they play over is the mock's own Cockpit, since
// the preview a pull request deployed is not reachable from a browser tab, so each names a part that page has.
const hubSteps: readonly WalkStep[] = [
  { say: "The window is connected to Fleet and lists its work", look: { role: "region", name: "Jobs and Sessions" } },
  { say: "Restart Fleet now, with this window open. Activity keeps drawing", look: { role: "region", name: "Activity" } },
  { say: "The same page is still open and the list is back, with no reload", look: { role: "region", name: "Jobs and Sessions" } },
];

const bayHistorySteps: readonly WalkStep[] = [
  { say: "The bays, each with its holder", look: { role: "region", name: "Bays" } },
  { say: "A bay's pane draws its leases and checkouts, newest first", look: { role: "region", name: "Activity" } },
];

const READ_AT = "2026-10-16T17:02:11Z";

const planUsageRequests: readonly ApiStep[] = [
  {
    id: "first",
    say: "Read the plan windows",
    method: "GET",
    path: "/plan-usage",
    response: {
      status: 200,
      ms: 412,
      body: `{\n  "windows": [\n    { "kind": "five_hour", "used": 0.42, "resets_at": "2026-10-16T21:00:00Z" },\n    { "kind": "week", "used": 0.31, "resets_at": "2026-10-20T00:00:00Z" }\n  ],\n  "read_at": "${READ_AT}"\n}`,
    },
    look: "read_at is there, and it is a time inside the last minute",
  },
  {
    id: "again",
    say: "Read them again straight away",
    method: "GET",
    path: "/plan-usage",
    response: {
      status: 200,
      ms: 3,
      body: `{\n  "windows": [\n    { "kind": "five_hour", "used": 0.42, "resets_at": "2026-10-16T21:00:00Z" },\n    { "kind": "week", "used": 0.31, "resets_at": "2026-10-20T00:00:00Z" }\n  ],\n  "read_at": "${READ_AT}"\n}`,
    },
    look: "The same read_at, and the answer came back in milliseconds: Fleet served the read it held",
  },
  {
    id: "later",
    say: "Read them after a minute has passed",
    method: "GET",
    path: "/plan-usage",
    response: {
      status: 200,
      ms: 387,
      body: `{\n  "windows": [\n    { "kind": "five_hour", "used": 0.43, "resets_at": "2026-10-16T21:00:00Z" },\n    { "kind": "week", "used": 0.31, "resets_at": "2026-10-20T00:00:00Z" }\n  ],\n  "read_at": "2026-10-16T17:03:14Z"\n}`,
    },
    look: "read_at has moved on, so the provider was called once for this read",
  },
];

/** The walk the agent wrote for a pull request, by its key. A pull request with none has nothing to walk. */
const plans: Readonly<Record<string, WalkPlan>> = {
  [`${KEY}#1907`]: { kind: "web", slot: 1, steps: hubSteps },
  [`${KEY}#1899`]: { kind: "api", slot: 3, requests: planUsageRequests },
  [`${KEY}#1903`]: { kind: "web", slot: 2, steps: bayHistorySteps },
};

export function walkSeed(): WalkSeed {
  return {
    plans,
    memory: [
      { id: "place-fleet", repository: KEY, area: "crates/fleet", place: { how: "pull", address: "", command: "armada fleet --dev", api: "http://localhost:7420" } },
      { id: "place-pocket", repository: "mele-labs/pocket", area: "src", place: { how: "deployed", address: "https://pocket.staging.armada.dev", command: "", api: "" } },
    ],
  };
}
