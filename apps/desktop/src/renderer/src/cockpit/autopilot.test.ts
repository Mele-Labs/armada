import { describe, expect, it } from "vitest";
import type { Autopilot, IssueList } from "@armada/protocol";

import { clockOf, dollars, cappedOf, localNow, nextUpOf, readingOf, settingsFrom, setAsideOf } from "./autopilot";

const NOW = Date.parse("2026-10-12T05:00:00.000Z");

function autopilot(over: Partial<Autopilot> = {}): Autopilot {
  return {
    manifest_id: "armada",
    instruction: "work through the open issues that don't need design decisions",
    at_once: 2,
    until: "2026-10-12T07:00:00.000Z",
    spend_cap_micros: 30_000_000,
    state: "running",
    started_at: "2026-10-12T03:00:00.000Z",
    on_autopilot: { backlog: true, running: false },
    decisions: [],
    spent_micros: 12_400_000,
    sessions_started: 3,
    at_work: 2,
    picks: [
      { pick_id: "p1", issue_reference: "#1850", issue_title: "Log viewer drops the last line", issue_url: "u", session_id: "s1", state: "started", reason: "clear", at: "" },
      { pick_id: "p2", issue_reference: "#1820", issue_title: "Flaky scout cost test", issue_url: "u", session_id: "s2", state: "started", reason: "clear", at: "" },
      { pick_id: "p3", issue_reference: "#1834", issue_title: "Wrong label on Settings", issue_url: "u", session_id: "s3", state: "started", reason: "clear", at: "" },
      { pick_id: "p4", issue_reference: "#1802", issue_title: "Which settings file wins", issue_url: "u", state: "set_aside", reason: "A design call.", at: "" },
      { pick_id: "p5", issue_reference: "#1790", issue_title: "Old one", issue_url: "u", state: "withdrawn", reason: "clear", at: "" },
    ],
    ...over,
  };
}

describe("which cap paused it", () => {
  it("names the cap that paused it, and says nothing of a person's own Pause", () => {
    expect(cappedOf(autopilot({ state: "paused", capped_by: "sessions" }))).toBe("Job cap reached");
    expect(cappedOf(autopilot({ state: "paused", capped_by: "spend" }))).toBe("spend cap reached");
    expect(cappedOf(autopilot({ state: "paused", capped_by: "until" }))).toBe("time to stop passed");
    expect(cappedOf(autopilot({ state: "paused" }))).toBeUndefined();
    expect(cappedOf(autopilot())).toBeUndefined();
  });
});

describe("Next up", () => {
  const issues: IssueList = {
    manifest_id: "armada",
    fetched_at: "",
    issues: [
      { reference: "#1850", title: "Picked already", url: "u", labels: [], assignees: [], updated_at: "", excerpt: "", linked_pull_request: false, state: "open", fields: [], holders: [] },
      { reference: "#1849", title: "Fleet restart drops a lease", url: "u", labels: [], assignees: [], updated_at: "", excerpt: "", linked_pull_request: true, state: "open", fields: [], holders: [] },
      { reference: "#1853", title: "Copy button missing", url: "u", labels: [], assignees: [], updated_at: "", excerpt: "", linked_pull_request: false, state: "open", fields: [], holders: [] },
      { reference: "#1856", title: "Badge count off by one", url: "u", labels: [], assignees: [], updated_at: "", excerpt: "", linked_pull_request: false, state: "open", fields: [], holders: [] },
    ],
  };

  it("numbers what it can choose, leaves out what it picked, and passes over one with a pull request last", () => {
    expect(nextUpOf(autopilot(), issues).map((one) => [one.order, one.reference, one.note])).toEqual([
      [1, "#1853", undefined],
      [2, "#1856", undefined],
      [undefined, "#1849", "has a pull request"],
    ]);
    expect(nextUpOf(autopilot(), undefined)).toEqual([]);
  });

  it("is bounded however long the backlog", () => {
    const many = { ...issues, issues: Array.from({ length: 40 }, (_, at) => ({ ...issues.issues[2]!, reference: `#${2000 + at}` })) };
    expect(nextUpOf(autopilot(), many)).toHaveLength(8);
  });

  it("leaves the set-aside picks to their own siding", () => {
    expect(setAsideOf(autopilot()).map((pick) => pick.issue_reference)).toEqual(["#1802"]);
  });
});

describe("the start form", () => {
  it("draws Fleet's reading over the words each setting came from, blank where the sentence said nothing", () => {
    const quoted = { instruction: "x", at_once: "two at a time", spend_cap_micros: "$30 max", until: " " };
    const reading = readingOf({ instruction: "x", at_once: 2, spend_cap_micros: 30_000_000, until: "2026-10-12T07:00:00.000Z", quoted });
    expect(reading.fields).toEqual({ atOnce: "2", until: clockOf("2026-10-12T07:00:00.000Z"), spend: "30", sessions: "" });
    // A blank quote is no quote: the form says the sentence did not say it.
    expect(reading.quoted).toEqual({ atOnce: "two at a time", spend: "$30 max" });
    expect(readingOf({ instruction: "x", at_once: 1 }).quoted).toEqual({});
  });

  it("sends the checked boxes, and never a word Fleet does not know", () => {
    const fields = { atOnce: "1", until: "", spend: "", sessions: "" };
    const boxes = { work: new Set(["running"]), decisions: new Set(["land", "plan", "destructive"]) };
    const read = settingsFrom("x", fields, NOW, boxes);
    expect(read.ok && read.settings).toMatchObject({ on_autopilot: { backlog: false, running: true }, decisions: ["plan", "land"] });
    const bare = settingsFrom("x", fields, NOW);
    expect(bare.ok && "on_autopilot" in bare.settings).toBe(false);
  });

  it("takes what the person confirmed, and refuses a field it cannot read", () => {
    const read = settingsFrom(" work it ", { atOnce: "2", until: "7am", spend: "$30.50", sessions: "" }, NOW);
    expect(read.ok).toBe(true);
    if (!read.ok) return;
    expect(read.settings).toMatchObject({ instruction: "work it", at_once: 2, spend_cap_micros: 30_500_000 });
    expect(read.settings.session_cap).toBeUndefined();
    const until = new Date(read.settings.until!);
    expect([until.getHours(), until.getMinutes()]).toEqual([7, 0]);
    expect(until.getTime()).toBeGreaterThan(NOW);
    expect(settingsFrom("x", { atOnce: "0", until: "", spend: "", sessions: "" }, NOW)).toEqual({ ok: false, why: "At once is a whole number of Jobs, 1 or more." });
    expect(settingsFrom("x", { atOnce: "1", until: "soon", spend: "", sessions: "" }, NOW)).toEqual({ ok: false, why: "Until is a time of day, like 07:00." });
    expect(settingsFrom("x", { atOnce: "1", until: "", spend: "lots", sessions: "" }, NOW)).toEqual({ ok: false, why: "Spend is an amount in dollars, like 30." });
  });

  it("writes money and the person's own clock as they read them", () => {
    expect(dollars(30_000_000)).toBe("$30");
    expect(dollars(4_120_000)).toBe("$4.12");
    expect(localNow(NOW)).toMatch(/^2026-10-1\dT\d\d:\d\d:\d\d[+-]\d\d:\d\d$/);
  });
});
