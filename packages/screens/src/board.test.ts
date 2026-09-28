// The Board's arithmetic, case by case.
//
// **Every case here is a sentence `board.ts` already claims.** Its comments say
// which tab a status lands in and why the order of two tests matters; this is
// that document executed. Where a test's name reads as a claim about the
// product rather than about a function, that is deliberate — the claim is what
// is being pinned, and the function is how it is spelled today.
//
// The statuses are the wire's own, read through `JOB_LIFECYCLE`. Nothing here
// lists which are terminal: the registry says, and a test that retyped the list
// would go on passing after the registry changed.

import { describe, expect, it } from "vitest";

import {
  BOARD_COLUMNS,
  columnsFor,
  DEFAULT_SORT,
  ofPicked,
  sectionOf,
  sectionsOf,
  needsYou,
  sorted,
  tabOf,
  taskBarSegmentsOf,
  taskFigureOf,
  TASKS_COLUMN,
} from "./board";

import type { JobSummary } from "@armada/protocol";

/** A row with everything the Board reads, and nothing it does not. */
function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    handle: "12-a-job",
    title: "Coalesce concurrent token refreshes",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    ...over,
  };
}

describe("which Jobs the Board lists", () => {
  const ours = job({ id: "ours", owner_manifest_id: "armada" });
  const theirs = job({ id: "theirs", owner_manifest_id: "ynap" });

  it("lists only the picked repository's Jobs", () => {
    expect(ofPicked([ours, theirs], { manifest: { id: "armada" } })).toEqual([ours]);
  });

  it("lists no Job for a picked repository that has no Manifest yet", () => {
    expect(ofPicked([ours, theirs], {})).toEqual([]);
  });

  it("lists every repository's Jobs on All repositories, which is no one picked", () => {
    expect(ofPicked([ours, theirs], null)).toEqual([ours, theirs]);
  });

  it("keeps every Job on All, including one whose repository is no longer served", () => {
    const orphan = job({ id: "01M130Y1380016YK5S0JXBXDQ7", owner_manifest_id: "gone" });
    expect(ofPicked([ours, theirs, orphan], null)).toEqual([ours, theirs, orphan]);
    expect(ofPicked([ours, theirs, orphan], { manifest: { id: "armada" } })).toEqual([ours]);
  });

  it("lists every Job when Fleet has listed no repository to pick", () => {
    expect(ofPicked([], null)).toEqual([]);
    expect(ofPicked([ours], null)).toEqual([ours]);
  });
});

describe("which tab a job is in", () => {
  it("puts every terminal status under Finished", () => {
    for (const status of ["completed_success", "completed_failed", "killed", "rejected", "superseded"]) {
      expect(tabOf(job({ status })), status).toBe("finished");
    }
  });

  it("puts a reclaimed job under Cleared, whichever terminal status it stopped at, and not under Finished", () => {
    // `#570`: a bulk clear used to delete the row it was asked to keep. The
    // fix is a row fact, not a registry one — `reclaimed_at` is not on any
    // lifecycle row, so this is the one rule in the function that is not read
    // off `JOB_LIFECYCLE`.
    for (const status of ["completed_success", "killed", "rejected"]) {
      expect(
        tabOf(job({ status, reclaimed_at: "2026-09-09T00:00:00.000Z" })),
        status,
      ).toBe("cleared");
    }
  });

  it("puts a job somebody has taken over under Running, not Needs you", () => {
    // `piloted` is `Working` and its actor is a `Person`. Asking the mode first
    // is the whole of why a piloted job does not read as one waiting on you.
    expect(tabOf(job({ status: "piloted" }))).toBe("running");
  });

  it("puts the gates a person answers under Needs you", () => {
    for (const status of ["awaiting_approval", "awaiting_attestation", "awaiting_review", "escalated"]) {
      expect(tabOf(job({ status })), status).toBe("needs-you");
    }
  });

  it("sorted a newly minted status without a line being written here", () => {
    // `awaiting_repair` landed in `job-statuses.toml` carrying `Waited on` and
    // `Person`, and the tab followed from the registry rather than from a list
    // — which is the property the four positive rules exist for. The Board's
    // filter cost nothing but the codegen run, and this case is what says so.
    expect(tabOf(job({ status: "awaiting_repair" }))).toBe("needs-you");
  });

  it("puts a queued job under Queued", () => {
    expect(tabOf(job({ status: "queued" }))).toBe("queued");
  });

  it("lifts a running job whose drone is asking into Needs you", () => {
    // The rule that is not a lifecycle row. Without it a question sits under
    // Running and nobody sees it until they open that job.
    expect(tabOf(job({ status: "running" }))).toBe("running");
    expect(tabOf(job({ status: "running", asking: true }))).toBe("needs-you");
  });

  it("answers null for a status this build has never heard of", () => {
    // Not a fifth tab, and not silently Queued. The counts stop summing, which
    // is what makes a registry change visible instead of invisible.
    expect(tabOf(job({ status: "translated_into_greek" }))).toBeNull();
  });

  it("draws the unplaceable under Other rather than dropping it", () => {
    // `sectionOf` is where that falls out now: a status no tab claims has no
    // tab to be counted under, and the section is what keeps the row on screen.
    expect(sectionOf(job({ status: "translated_into_greek" }))).toBe("other");
  });

  it("reads needsYou off the same rule the tab uses", () => {
    expect(needsYou(job({ status: "awaiting_review" }))).toBe(true);
    expect(needsYou(job({ status: "running" }))).toBe(false);
    expect(needsYou(job({ status: "running", asking: true }))).toBe(true);
  });
});

describe("where the list opens", () => {
  it("sorts critical first", () => {
    expect(DEFAULT_SORT).toBe("critical_first");
  });
});

describe("the order", () => {
  const early = job({ id: "a", status: "running", created_at: "2026-08-30T09:00:00Z" });
  const late = job({ id: "b", status: "running", created_at: "2026-08-31T09:00:00Z" });
  const waiting = job({ id: "c", status: "awaiting_review", created_at: "2026-08-31T12:00:00Z" });

  it("lifts the needs-you cluster, then goes oldest first inside every group", () => {
    expect(sorted([late, early, waiting], "critical_first").map((row) => row.id)).toEqual([
      "c",
      "a",
      "b",
    ]);
  });

  it("lifts nothing under Oldest first, and orders the same jobs by age alone", () => {
    expect(sorted([late, waiting, early], "oldest_first").map((row) => row.id)).toEqual([
      "a",
      "b",
      "c",
    ]);
  });

  it("sorts a job whose date will not parse last, never first", () => {
    const corrupt = job({ id: "z", status: "running", created_at: "not a date" });
    expect(sorted([corrupt, late, early], "oldest_first").map((row) => row.id)).toEqual([
      "a",
      "b",
      "z",
    ]);
  });

  it("breaks a tie on the id, so the order does not shuffle between renders", () => {
    const first = job({ id: "aaa", status: "running", created_at: "2026-08-31T09:00:00Z" });
    const second = job({ id: "bbb", status: "running", created_at: "2026-08-31T09:00:00Z" });
    expect(sorted([second, first], "oldest_first").map((row) => row.id)).toEqual(["aaa", "bbb"]);
  });

  it("does not reorder the caller's array", () => {
    const held = [late, early];
    sorted(held, "oldest_first");
    expect(held.map((row) => row.id)).toEqual(["b", "a"]);
  });
});

describe("the All tab's sections", () => {
  it("draws what needs you, what runs, what waits and what is over, in that order", () => {
    const rows = [
      job({ id: "a", status: "completed_success" }),
      job({ id: "b", status: "queued" }),
      job({ id: "c", status: "running" }),
      job({ id: "d", status: "awaiting_approval" }),
    ];
    expect(sectionsOf(rows).map((section) => section.id)).toEqual([
      "needs-you",
      "running",
      "queued",
      "done",
    ]);
  });

  it("puts a cleared job under Done with the finished ones", () => {
    expect(sectionOf(job({ status: "completed_success", reclaimed_at: "2026-09-01T00:00:00Z" }))).toBe(
      "done",
    );
  });

  // Overview 28 (#1092): a Job that ended without succeeding stood nowhere to be found —
  // Done collapsed by default, and Overview dropped it outright. Recently ended is the fix.
  it.each(["killed", "completed_failed", "rejected"])(
    "puts a %s job under Recently ended rather than Done",
    (status) => {
      expect(sectionOf(job({ status }))).toBe("recently-ended");
    },
  );

  it("leaves a job that succeeded, or that nothing is owed on, under Done", () => {
    expect(sectionOf(job({ status: "completed_success" }))).toBe("done");
    expect(sectionOf(job({ status: "superseded" }))).toBe("done");
  });

  it("moves a Recently ended job back under Done once it is cleared", () => {
    expect(sectionOf(job({ status: "killed", reclaimed_at: "2026-09-01T00:00:00Z" }))).toBe("done");
  });

  it("draws Recently ended above Done and below Queued", () => {
    const rows = [
      job({ id: "a", status: "killed" }),
      job({ id: "b", status: "completed_success" }),
      job({ id: "c", status: "queued" }),
    ];
    expect(sectionsOf(rows).map((section) => section.id)).toEqual(["queued", "recently-ended", "done"]);
  });

  it("draws no section with nothing in it", () => {
    expect(sectionsOf([job({ status: "running" })]).map((section) => section.id)).toEqual(["running"]);
  });

  it("keeps the order each section was handed", () => {
    const rows = [job({ id: "second", status: "running" }), job({ id: "first", status: "running" })];
    expect(sectionsOf(rows)[0]?.jobs.map((row) => row.id)).toEqual(["second", "first"]);
  });

  it("keeps a job no tab claims, under a label of its own", () => {
    expect(sectionOf(job({ status: "not_a_status_the_registry_has" }))).toBe("other");
  });
});

describe("the column that says where a job came from", () => {
  // #1362. Fixed and not conditional: `origin` is `NOT NULL`, so there is no
  // board where the track would be reserved for nothing.
  it("is drawn on every board", () => {
    expect(columnsFor([], null, false)).toContain("Dispatched by");
    expect(BOARD_COLUMNS.at(-1)).toBe("Dispatched by");
  });

  it("comes after the three a person scans, and before Repository and Tasks", () => {
    const columns = columnsFor([job({ tasks: { done: 1, working: 0, open: 1, dropped: 0 } })], null, false);
    expect(columns.indexOf("Dispatched by")).toBeGreaterThan(columns.indexOf("Run time"));
    expect(columns.indexOf("Dispatched by")).toBeLessThan(columns.indexOf(TASKS_COLUMN));
  });
});

describe("a Board row's tasks", () => {
  it("names the Tasks column only where some row has a plan", () => {
    expect(columnsFor([job()], null, false)).toEqual(BOARD_COLUMNS);
    expect(columnsFor([job({ tasks: { done: 1, working: 1, open: 1, dropped: 0 } })], null, false)).toEqual([
      ...BOARD_COLUMNS,
      TASKS_COLUMN,
    ]);
  });

  it("draws done segments first, then the one working, then what is open", () => {
    expect(taskBarSegmentsOf({ done: 2, working: 1, open: 1, dropped: 1 })).toEqual([
      "done",
      "done",
      "working",
      "open",
    ]);
  });

  it("counts done over tasks not dropped", () => {
    expect(taskFigureOf({ done: 1, working: 1, open: 1, dropped: 1 })).toBe("1 of 3");
  });
});
