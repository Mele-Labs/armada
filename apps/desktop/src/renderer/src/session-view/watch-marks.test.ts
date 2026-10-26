import { expect, test } from "vitest";
import type { WatchReading, WatchSummary } from "@armada/protocol";

import { escalation, notOursParts, watchesShown, watchSaid } from "./watch-marks";

const ended: WatchReading = {
  kind: "read",
  at: "2026-10-26T14:02:31.000Z",
  state: "failed",
  finished: true,
  says: "provision failed: cloud lookup for region qa-east returned no account",
  link: "https://ci.example.com/runs/5120",
  parts: [
    { name: "build", state: "passed" },
    { name: "provision", state: "failed" },
  ],
};

const deploy = (state: WatchSummary["state"], parts = ended.parts): WatchSummary => ({
  id: "w-1",
  session_id: "s-1",
  name: "QA deploy",
  command: ["scripts/watch/deploy-state"],
  every_seconds: 60,
  gives_up_after_seconds: 7200,
  state,
  live: false,
  started_at: "2026-10-26T13:40:00.000Z",
  readings: [{ ...ended, ...(parts === undefined ? {} : { parts }) }],
  set_aside: [{ part: "provision", sentence: "a cloud lookup this change does not touch" }],
});

test("a not ours mark names the set-aside parts that failed, and no other", () => {
  expect(notOursParts(deploy("not_ours"))).toEqual(["provision"]);
  expect(watchSaid(deploy("not_ours"))).toBe("Not ours: provision failed");
  expect(watchSaid(deploy("failed"))).toBe("Failed");
  const retried = deploy("passed", [{ name: "build", state: "passed" }, { name: "provision", state: "passed" }]);
  expect(notOursParts(retried)).toEqual([]);
});

test("escalate gives what failed, when, the run, the error and the link, and nothing more", () => {
  expect(escalation(deploy("not_ours")).split("\n")).toEqual([
    "provision failed in QA deploy.",
    "When: 2026-10-26 14:02 UTC",
    "Run: QA deploy",
    "Error: provision failed: cloud lookup for region qa-east returned no account",
    "Link: https://ci.example.com/runs/5120",
  ]);
});

test("an ended not ours watch stays listed while Needs you asks about it, and hides once settled", () => {
  const item = { id: "watch_not_ours:w-1", text: "QA deploy: provision failed", since: "2026-10-26T14:02:31.000Z", source: "watch_not_ours" as const, act: { kind: "answer" as const, target: "w-1" } };
  const stopped = { ...deploy("stopped"), id: "w-2" };
  const ids = (shown: WatchSummary[]) => shown.map((one) => one.id);
  expect(ids(watchesShown([deploy("not_ours"), stopped], [item], false))).toEqual(["w-1"]);
  expect(ids(watchesShown([deploy("not_ours"), stopped], [], false))).toEqual([]);
  expect(ids(watchesShown([deploy("not_ours"), stopped], [], true))).toEqual(["w-1", "w-2"]);
});
