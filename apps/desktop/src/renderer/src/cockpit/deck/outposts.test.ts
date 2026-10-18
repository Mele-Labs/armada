// The outposts as Fleet's own entries draw them: `session.called` read as the wire carries it, not as the mock builds it.

import type { TimelineEvent } from "@armada/protocol";
import { expect, test } from "vitest";

import { FRESH_MS } from "./doing";
import type { DroneIn } from "./layout";
import { outpostsOf, withCalling } from "./outposts";

const NOW = Date.parse("2026-10-17T12:00:00Z");

/** An entry as `get_fleet_timeline` and `timeline.appended` serve it: the JSON Fleet writes, parsed. */
const served = (id: number, ago: number, service: string, line: string, session = "s1"): TimelineEvent =>
  JSON.parse(
    `{"id":${id},"at":"${new Date(NOW - ago).toISOString()}","kind":"session.called","subject":{"kind":"session","id":"${session}"},` +
      `"title":"Land it","tone":"run","says":"Called ${service}","detail":"${line}","call":{"service":"${service}","line":"${line}"},"manifest_id":"armada","live":true}`,
  ) as TimelineEvent;

const drone = (key: string, working = true): DroneIn => ({
  key,
  kind: "session",
  title: key,
  root: "/r",
  said: "",
  hue: "calm",
  spin: working,
  hover: working,
  ping: false,
  stuck: false,
  flare: false,
  working,
  touches: [],
  shown: true,
});
const none = () => undefined;

test("a service gets an outpost with the first call to it, the fixed three first and the rest by name", () => {
  const events = [
    served(1, 50_000, "figma", "get_design_context"),
    served(2, 40_000, "web", "docs.rs"),
    served(3, 30_000, "tracker", "gh issue create"),
    served(4, 20_000, "github", "gh pr view"),
    served(5, 10_000, "playwright", "browser_click"),
  ];
  expect(outpostsOf(events, NOW).map((one) => one.service)).toEqual(["github", "tracker", "web", "figma", "playwright"]);
  expect(outpostsOf(events, NOW).map((one) => one.glyph)).toEqual(["github", "ticket", "globe", "plug", "plug"]);
});

test("an outpost's card lists the latest calls, newest first, each as the Session, the line and its age", () => {
  const [github] = outpostsOf([served(1, 60_000, "github", "gh pr view"), served(2, 5_000, "github", "gh run list")], NOW);
  expect(github?.lines).toHaveLength(2);
  expect(github?.lines[0]).toContain("Land it · gh run list");
  expect(github?.lines[1]).toContain("Land it · gh pr view");
});

test("a service nobody has called has no outpost, and a call after the clock is not yet", () => {
  expect(outpostsOf([], NOW)).toEqual([]);
  expect(outpostsOf([served(1, -10_000, "github", "gh pr view")], NOW)).toEqual([]);
});

test("a working drone whose latest call is fresh is calling that service, and a stale or idle one is not", () => {
  const events = [served(1, 10_000, "web", "docs.rs", "s1"), served(2, FRESH_MS + 1_000, "github", "gh pr view", "s2"), served(3, 5_000, "figma", "get_design_context", "s3")];
  const drones = [drone("session:s1"), drone("session:s2"), drone("session:s3", false), drone("session:s4")];
  expect(withCalling(drones, events, NOW, none).map((one) => one.calling)).toEqual(["web", undefined, undefined, undefined]);
});

test("a drone's latest call is the one it is calling", () => {
  const events = [served(1, 60_000, "github", "gh pr view"), served(2, 5_000, "web", "docs.rs")];
  expect(withCalling([drone("session:s1")], events, NOW, none)[0]?.calling).toBe("web");
});
