// What the Cockpit's filters keep of the timeline's lanes: Active every lane that is not over, the
// subjects that need him among them as their own group; Done what is over.

import { expect, test } from "vitest";

import { STANDINGS, lanesFor, lanesOf, type FleetEvent } from "./fleet-events";

const NOW = Date.parse("2026-10-13T12:00:00Z");
const at = (minutesAgo: number) => new Date(NOW - minutesAgo * 60_000).toISOString();

const event = (id: string, job: string, over: Partial<FleetEvent>): FleetEvent => ({
  id,
  at: at(1),
  kind: "job.step",
  subject: { kind: "job", id: job, title: job },
  tone: "run",
  says: "says",
  repo: "armada",
  ...over,
});

const lanes = lanesOf(
  [
    event("1", "asking", { tone: "ask" }),
    event("2", "failing", { tone: "bad" }),
    event("3", "quiet", { live: true, at: at(30) }),
    event("4", "moving", { live: true }),
    event("5", "waiting", {}),
    event("6", "finished", { over: true }),
  ],
  NOW,
);

test("Active keeps every lane that is not over, and the ones that need him stand as their own group", () => {
  expect(lanesFor(lanes, "running").map((one) => one.standing).sort()).toEqual(["ask", "bad", "idle", "run", "stuck"]);
  expect(STANDINGS.find((one) => one.id === "ask")?.said).toBe("Needs you");
  expect(lanesFor(lanes, "running").find((one) => one.standing === "ask")?.key).toBe("job:asking");
});

test("Done keeps what is over", () => {
  expect(lanesFor(lanes, "done").map((one) => one.key)).toEqual(["job:finished"]);
});
