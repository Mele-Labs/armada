// What the Walk tool would write for three of the review mock's pull requests, and the places Armada already remembers.
// **One of each way a place is found**: #1907 has a preview link a bot posted, so its walk is a web walk against that
// address; #1899 touches Fleet, whose place Armada remembers, so its walk is a run of requests against the API there;
// #1903 has neither and asks. The steps are written by hand, as the pull requests' hunks are, and name their targets
// the way a `show_window` step does: by role and name.

import type { RememberedPlace, ReviewRequest, WalkStep } from "@armada/protocol";

import type { WalkSeed } from "./review-seed";
import type { MockResponse } from "./review-walk-runner";

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
const windows = (used: number) => `{\n  "windows": [\n    { "kind": "five_hour", "used": ${used}, "resets_at": "2026-10-16T21:00:00Z" },\n    { "kind": "week", "used": 0.31, "resets_at": "2026-10-20T00:00:00Z" }\n  ],\n  "read_at": "`;

const planUsageRequests: readonly ReviewRequest[] = [
  { say: "Read the plan windows", method: "GET", path: "/plan-usage", look: "read_at is there, and it is a time inside the last minute" },
  { say: "Read them again straight away", method: "GET", path: "/plan-usage", look: "The same read_at, and the answer came back in milliseconds: Fleet served the read it held" },
  { say: "Read them after a minute has passed", method: "GET", path: "/plan-usage", look: "read_at has moved on, so the provider was called once for this read" },
];

const planUsageResponses: readonly MockResponse[] = [
  { status: 200, ms: 412, body: `${windows(0.42)}${READ_AT}"\n}` },
  { status: 200, ms: 3, body: `${windows(0.42)}${READ_AT}"\n}` },
  { status: 200, ms: 387, body: `${windows(0.43)}2026-10-16T17:03:14Z"\n}` },
];

/** The walk the agent wrote for a pull request, by its key. A pull request with none has nothing to walk. */
export const walkSeeds = (): Readonly<Record<string, WalkSeed>> => ({
  [`${KEY}#1907`]: { kind: "web", slot: 1, steps: hubSteps },
  [`${KEY}#1899`]: { kind: "api", slot: 3, requests: planUsageRequests, responses: planUsageResponses },
  [`${KEY}#1903`]: { kind: "web", slot: 2, steps: bayHistorySteps },
});

/** What Armada remembers before anything is answered. */
export const rememberedPlaces = (): readonly RememberedPlace[] => [
  { id: "place-fleet", manifest_id: "armada", repository: KEY, area: "crates/fleet", place: { how: "pull", command: "armada fleet --dev", api: "http://localhost:7420" } },
  { id: "place-pocket", manifest_id: "pocket", repository: "mele-labs/pocket", area: "src", place: { how: "deployed", address: "https://pocket.staging.armada.dev" } },
];
