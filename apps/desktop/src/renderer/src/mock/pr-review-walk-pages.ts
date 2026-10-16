// What the Walk tool would write for three of the review mock's pull requests, and the places Armada already remembers.
// **One of each way a place is found**: #1907 has a preview link a bot posted, so its walk is a web walk against that
// address; #1899 touches Fleet, whose place Armada remembers, so its walk is a run of requests against the API there;
// #1903 has neither and asks. The steps are written by hand, as the pull requests' hunks are, and name their targets
// the way a `show_window` step does: by role and name.

import type { ApiStep, WalkPlan, WalkSeed, WebStep } from "../pr-review/walk-model";

const KEY = "mele-labs/armada";

const hubSteps: readonly WebStep[] = [
  { id: "link", say: "The window is connected to Fleet", verb: "look", role: "status", name: "Fleet link" },
  { id: "restart", say: "Restart Fleet with this window open", verb: "press", role: "button", name: "Restart Fleet" },
  { id: "back", say: "The link reads reconnecting and then connected, with no reload", verb: "look", role: "status", name: "Fleet link" },
  { id: "same", say: "The Cockpit is the page that was open, as it was", verb: "look", role: "region", name: "Jobs and Sessions" },
];

const bayHistorySteps: readonly WebStep[] = [
  { id: "bay", say: "Open a bay's pane", verb: "press", role: "button", name: "Bay 3" },
  { id: "history", say: "Open its history", verb: "press", role: "tab", name: "History" },
  { id: "entries", say: "Leases and checkouts of this bay, newest first", verb: "look", role: "list", name: "History" },
  { id: "cut", say: "A list cut at its limit says so", verb: "look", role: "note", name: "Older entries exist" },
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
  [`${KEY}#1899`]: { kind: "api", slot: 3, steps: planUsageRequests },
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
