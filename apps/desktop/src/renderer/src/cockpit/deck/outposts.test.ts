// The outposts as Fleet's own entries draw them: `session.called` read as the wire carries it, not as the mock builds it.

import type { TimelineEvent } from "@armada/protocol";
import { expect, test } from "vitest";

import { FRESH_MS } from "./doing";
import type { DroneIn } from "./layout";
import { outpostsOf, withCalling } from "./outposts";

const NOW = Date.parse("2026-10-17T12:00:00Z");

/** What Fleet calls each service in a call, as a row says it. */
const NAMED: Record<string, string> = { forge: "the forge", tracker: "the issue tracker", web: "the web", figma: "Figma", playwright: "Playwright" };

/** An entry as `get_fleet_timeline` and `timeline.appended` serve it: the JSON Fleet writes, parsed. */
const served = (id: number, ago: number, service: string, line: string, session = "s1"): TimelineEvent =>
  JSON.parse(
    `{"id":${id},"at":"${new Date(NOW - ago).toISOString()}","kind":"session.called","subject":{"kind":"session","id":"${session}"},` +
      `"title":"Land it","tone":"run","says":"Called ${NAMED[service]}","detail":"${line}","call":{"service":"${service}","name":"${NAMED[service]}","line":"${line}"},"manifest_id":"armada","live":true}`,
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
    served(4, 20_000, "forge", "gh pr view"),
    served(5, 10_000, "playwright", "browser_click"),
  ];
  expect(outpostsOf(events, NOW).map((one) => one.service)).toEqual(["forge", "tracker", "web", "figma", "playwright"]);
  expect(outpostsOf(events, NOW).map((one) => one.glyph)).toEqual(["forge", "ticket", "globe", "plug", "plug"]);
});

test("an outpost is labelled with the newest call's name, the article dropped and the first letter capitalised", () => {
  const outposts = outpostsOf([served(1, 9_000, "web", "docs.rs"), served(2, 8_000, "tracker", "list_issues"), served(3, 7_000, "figma", "get_design_context"), served(4, 6_000, "forge", "gh pr view")], NOW);
  expect(outposts.map((one) => one.name)).toEqual(["Forge", "Issue tracker", "Web", "Figma"]);
  const renamed = { ...served(5, 1_000, "forge", "gh pr view"), call: { service: "forge", name: "Another forge", line: "gh pr view" } };
  expect(outpostsOf([served(1, 9_000, "forge", "gh pr view"), renamed], NOW)[0]?.name).toBe("Another forge");
});

test("an outpost's card lists the latest calls, newest first, each as the Session, the line and its age", () => {
  const [forge] = outpostsOf([served(1, 60_000, "forge", "gh pr view"), served(2, 5_000, "forge", "gh run list")], NOW);
  expect(forge?.lines).toHaveLength(2);
  expect(forge?.lines[0]).toContain("Land it · gh run list");
  expect(forge?.lines[1]).toContain("Land it · gh pr view");
});

test("a service nobody has called has no outpost, and a call after the clock is not yet", () => {
  expect(outpostsOf([], NOW)).toEqual([]);
  expect(outpostsOf([served(1, -10_000, "forge", "gh pr view")], NOW)).toEqual([]);
});

test("a working drone whose latest call is fresh is calling that service, and a stale or idle one is not", () => {
  const events = [served(1, 10_000, "web", "docs.rs", "s1"), served(2, FRESH_MS + 1_000, "forge", "gh pr view", "s2"), served(3, 5_000, "figma", "get_design_context", "s3")];
  const drones = [drone("session:s1"), drone("session:s2"), drone("session:s3", false), drone("session:s4")];
  expect(withCalling(drones, events, NOW, none).map((one) => one.calling)).toEqual(["web", undefined, undefined, undefined]);
});

test("a drone's latest call is the one it is calling", () => {
  const events = [served(1, 60_000, "forge", "gh pr view"), served(2, 5_000, "web", "docs.rs")];
  expect(withCalling([drone("session:s1")], events, NOW, none)[0]?.calling).toBe("web");
});
