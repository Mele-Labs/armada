import { describe, expect, it } from "vitest";
import type { Autopilot, JobSummary } from "@armada/protocol";

import type { Item } from "../Dashboard";
import { withAutopilot } from "./autopilot-metro";
import type { Line, Metro } from "./metro";

const job = (id: string, over: Partial<JobSummary> = {}) => ({ id, status: "running", origin: "dispatched", started_at: "2026-10-12T03:01:00Z", ...over }) as JobSummary;

function line(key: string, of: JobSummary, reach: number): Line {
  return {
    key,
    item: { key, job: of } as Item,
    bay: reach === 0 ? undefined : 1,
    title: key,
    session: false,
    tone: reach === 0 ? "idle" : "run",
    reach,
    stops: reach === 0 ? [{ tone: "idle", card: { tone: "idle", pill: "waiting", title: "Waiting for a bay", rows: [], acts: [] } }] : [],
    open: undefined,
    wants: undefined,
  };
}

const METRO: Metro = {
  harbours: [{ manifest: "armada", name: "armada", lines: [line("mine", job("j2"), 2), line("by-hand", job("j9"), 2)], outside: [] }],
  waiting: [line("picked", job("j1", { status: "queued", started_at: undefined }), 0), line("queued-by-hand", job("j8", { status: "queued", started_at: undefined }), 0)],
  counts: [[["idle", 2]], [], [["run", 2]], [], [], []],
  bays: { held: 2, free: 0, closed: 0, strip: [], names: [] },
  main: undefined,
  review: false,
};

const AUTOPILOT = {
  state: "running",
  picks: [
    { pick_id: "p1", issue_number: 1850, issue_title: "Log viewer drops the last line", issue_url: "u", job_id: "j1", state: "started", reason: "One file.", at: "" },
    { pick_id: "p2", issue_number: 1820, issue_title: "Flaky test", issue_url: "u", job_id: "j2", state: "started", reason: "Clear.", at: "" },
  ],
} as Autopilot;

describe("Autopilot on the Bays metro", () => {
  it("marks its lines, and moves a pick that has not started to Next up with a dashed stop", () => {
    const metro = withAutopilot(METRO, [AUTOPILOT]);
    expect(metro.harbours[0]!.lines.map((one) => [one.key, one.autopilot === true])).toEqual([
      ["mine", true],
      ["by-hand", false],
    ]);
    expect(metro.waiting.map((one) => one.key)).toEqual(["queued-by-hand"]);
    expect(metro.nextUp?.map((one) => one.key)).toEqual(["picked"]);
    const picked = metro.nextUp![0]!;
    expect(picked.reach).toBe(-1);
    expect(picked.stops.every((stop) => stop === undefined)).toBe(true);
    expect(picked.nextUp?.card.rows).toEqual([
      ["Issue", "#1850 Log viewer drops the last line"],
      ["Why", "One file."],
    ]);
    expect(metro.counts[0]).toEqual([["idle", 1]]);
  });

  it("draws nothing of a stopped one", () => {
    const metro = withAutopilot(METRO, [{ ...AUTOPILOT, state: "stopped" }]);
    expect(metro.nextUp).toEqual([]);
    expect(metro.waiting).toHaveLength(2);
    expect(metro.harbours[0]!.lines.some((one) => one.autopilot === true)).toBe(false);
  });
});
