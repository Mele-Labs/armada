// Fleet's account of where the time went, turned into what the page draws: the units change once, a figure nobody produced stays
// absent, and a commit run twice is two builds. The mock's own figures go to the wire and back unchanged.

import type {
  BuildFigures,
  HourFigures,
  RecordSplit,
  UsageHours as WireHours,
} from "@armada/protocol";
import { expect, test } from "vitest";

import { jobTimeOf } from "./usage-time-wire";
import { buildIdOf, buildsOf, hoursOf, runOf, splitOf, timeOf } from "./usage-time-wire";
import { jobTimeNow } from "./mock/job-time";
import { usageBuildsNow } from "./mock/usage-builds";
import { usageHoursNow } from "./mock/usage-hours";
import { wireHours, wireJobTime, wireTime } from "./mock/usage-time-wire";

const SINCE = "2026-11-02T00:00:00.000Z";
const HOUR = 3_600_000;

const split = (over: Partial<RecordSplit> = {}): RecordSplit => ({
  model: 0,
  tools: 0,
  checks: 0,
  subagents: 0,
  slot: 0,
  you: 0,
  pull_request: 0,
  stopped: 0,
  idle: 0,
  ...over,
});

const build = (over: Partial<BuildFigures> = {}): BuildFigures => ({
  commit: "4be7c19",
  started: "2026-11-02T09:00:00Z",
  source: "main",
  main_commit: "4be7c19",
  branches: [],
  turns: 100,
  jobs: [],
  split: split(),
  ...over,
});

const hours = (builds: BuildFigures[], entries: HourFigures[] = []): WireHours => ({
  since: SINCE,
  until: "2026-11-04T00:00:00Z",
  hours: entries,
  builds,
});

test("a split is milliseconds on the wire and seconds on the page, in every bucket", () => {
  expect(
    splitOf(
      split({
        model: 90_000,
        tools: 1500,
        checks: 3_600_000,
        subagents: 1,
        slot: 2000,
        you: 3000,
        pull_request: 4000,
        stopped: 5000,
        idle: 6000,
      }),
    ),
  ).toEqual({
    model: 90,
    tools: 1.5,
    checks: 3600,
    subagents: 0.001,
    slot: 2,
    you: 3,
    pull_request: 4,
    stopped: 5,
    idle: 6,
  });
});

test("the window's time carries seconds, the usual of each workflow by name, and a Job with no title shows its id", () => {
  const time = timeOf({
    since: SINCE,
    until: "2026-11-09T00:00:00Z",
    jobs: 2,
    total: split({
      model: 120_000,
    }),
    slowest: [
      {
        session_id: "01JTITLED",
        title: "Merge the lines",
        workflow: "Build a feature",
        began_at: SINCE,
        ended_at: SINCE,
        split: split({
          checks: 60_000,
        }),
      },
      {
        session_id: "01JUNTITLED",
        workflow: "Fix a bug",
        began_at: SINCE,
        ended_at: SINCE,
        split: split(),
      },
    ],
    usual: [
      {
        workflow: "Build a feature",
        jobs: 9,
        split: split({
          model: 2_400_000,
        }),
      },
      {
        workflow: "Fix a bug",
        jobs: 3,
        split: split({
          model: 1_320_000,
        }),
      },
    ],
  });
  expect(time.total.model).toBe(120);
  expect(
    time.jobs.map((one) => [
      one.id,
      one.title,
      one.split.checks,
    ]),
  ).toEqual([
    [
      "01JTITLED",
      "Merge the lines",
      60,
    ],
    [
      "01JUNTITLED",
      "01JUNTITLED",
      0,
    ],
  ]);
  expect(Object.keys(time.usual)).toEqual([
    "Build a feature",
    "Fix a bug",
  ]);
  expect(time.usual["Build a feature"]!.model).toBe(2400);
});

test("a Job's split is its own and the usual of its workflow, and none kept is none", () => {
  const kept = jobTimeOf(
    {
      session_id: "01J",
      time: {
        workflow: "Fix a bug",
        began_at: SINCE,
        ended_at: SINCE,
        spent: split({
          checks: 5000,
        }),
        usual: {
          workflow: "Fix a bug",
          jobs: 4,
          split: split({
            checks: 8000,
          }),
        },
      },
    },
    "Fix the label",
  );
  expect(kept?.job).toMatchObject({
    id: "01J",
    title: "Fix the label",
    workflow: "Fix a bug",
  });
  expect(kept?.job.split.checks).toBe(5);
  expect(kept?.usual?.checks).toBe(8);
  const unnamed = jobTimeOf(
    {
      session_id: "01J",
      time: {
        workflow: "",
        began_at: SINCE,
        ended_at: SINCE,
        spent: split(),
      },
    },
    undefined,
  );
  expect(unnamed?.job.title).toBe("01J");
  expect(unnamed?.usual).toBeUndefined();
  expect(
    jobTimeOf(
      {
        session_id: "01J",
      },
      "Fix the label",
    ),
  ).toBeUndefined();
});

test("a sparse hour is a gap in the 48, never a 0", () => {
  const drawn = hoursOf(
    hours(
      [],
      [
        {
          hour: 3,
          at: "",
          burn: [
            {
              name: "sonnet",
              units: 4,
            },
          ],
          model: 41,
          checks: 500,
        },
        {
          hour: 47,
          at: "",
          pull_request: 1400,
        },
      ],
    ),
  );
  expect(drawn).toHaveLength(48);
  expect(drawn[0]).toEqual({
    at: SINCE,
  });
  expect(drawn[3]).toEqual({
    at: new Date(Date.parse(SINCE) + 3 * HOUR).toISOString(),
    burn: [
      {
        name: "sonnet",
        units: 4,
      },
    ],
    model: 41,
    checks: 500,
  });
  expect("tools" in drawn[3]!).toBe(false);
  expect("pull_request" in drawn[3]!).toBe(false);
  expect(drawn[4]).toEqual({
    at: new Date(Date.parse(SINCE) + 4 * HOUR).toISOString(),
  });
  expect(drawn[47]!.pull_request).toBe(1400);
  expect(drawn.filter((one) => Object.keys(one).length > 1)).toHaveLength(2);
});

test("a figure a build did not produce stays absent, and the ones it did are the wire's own", () => {
  const [bare, full] = buildsOf(
    hours([
      build({
        started: "2026-11-02T09:00:00Z",
        burn: 81_200,
        model: 41.2,
        tools: 5.2,
        checks: 512,
        pull_request: 1380,
      }),
      build({
        started: "2026-11-02T10:00:00Z",
        commit: "aaaaaaa",
        main_commit: "aaaaaaa",
        turns: 0,
      }),
    ]),
    new Map(),
  );
  expect(full).toMatchObject({
    burn: 81_200,
    model: 41.2,
    tools: 5.2,
    checks: 512,
    pull_request: 1380,
  });
  for (const key of [
    "burn",
    "model",
    "tools",
    "checks",
    "pull_request",
    "usage",
  ])
    expect(key in bare!, key).toBe(false);
});

test("a commit run again later is a second build, with an id of its own that still starts with the commit", () => {
  const list = buildsOf(
    hours([
      build({
        started: "2026-11-02T09:00:00Z",
      }),
      build({
        started: "2026-11-02T15:30:00Z",
      }),
    ]),
    new Map(),
  );
  expect(list).toHaveLength(2);
  expect(new Set(list.map((one) => one.id)).size).toBe(2);
  for (const one of list) expect(one.id.slice(0, 7)).toBe("4be7c19");
  expect(list.map((one) => one.started)).toEqual([
    "2026-11-02T15:30:00Z",
    "2026-11-02T09:00:00Z",
  ]);
});

test("a build is a preview where it holds branches, and stands on its main commit", () => {
  const [preview, main] = buildsOf(
    hours([
      build({
        started: "2026-11-02T09:00:00Z",
      }),
      build({
        started: "2026-11-02T10:00:00Z",
        commit: "d03a6e1",
        source: "preview",
        main_commit: "4be7c19",
        branches: [
          "armada/a",
        ],
      }),
    ]),
    new Map(),
  );
  expect(preview).toMatchObject({
    commit: "4be7c19",
    branches: [
      "armada/a",
    ],
  });
  expect(preview!.id.slice(0, 7)).toBe("d03a6e1");
  expect(main).toMatchObject({
    commit: "4be7c19",
    branches: [],
  });
});

test("a build Fleet cannot place as main or as a preview is not drawn as main", () => {
  const list = buildsOf(
    hours([
      build({
        started: "2026-11-02T09:00:00Z",
        source: undefined,
        main_commit: undefined,
      }),
      build({
        started: "2026-11-02T10:00:00Z",
        source: "preview",
        branches: [],
      }),
      build({
        started: "2026-11-02T11:00:00Z",
        source: "preview",
        branches: [
          "armada/a",
        ],
        main_commit: undefined,
      }),
      build({
        started: "2026-11-02T12:00:00Z",
      }),
    ]),
    new Map(),
  );
  expect(list.map((one) => one.started)).toEqual([
    "2026-11-02T12:00:00Z",
  ]);
});

test("a build's tokens are the ones read for its run, and its run ends where the next began", () => {
  const ended = build({
    started: "2026-11-02T09:00:00Z",
    until: "2026-11-02T10:00:00Z",
  });
  const running = build({
    started: "2026-11-02T10:00:00Z",
  });
  expect(runOf(ended, "2026-11-04T00:00:00Z")).toEqual({
    since: "2026-11-02T09:00:00Z",
    until: "2026-11-02T10:00:00Z",
  });
  expect(runOf(running, "2026-11-04T00:00:00Z")).toEqual({
    since: "2026-11-02T10:00:00Z",
    until: "2026-11-04T00:00:00Z",
  });
  const usage = usageBuildsNow()[0]!.usage!;
  const [newest, oldest] = buildsOf(
    hours([
      ended,
      running,
    ]),
    new Map([
      [
        buildIdOf(running),
        usage,
      ],
    ]),
  );
  expect(newest!.usage).toBe(usage);
  expect("usage" in oldest!).toBe(false);
});

// The mock answers through the wire, so what it draws is what it drew before: the same figures, within the last bit a thousand
// does not keep.
const near = (a: unknown, b: unknown, path = ""): void => {
  if (typeof a === "number" && typeof b === "number") return expect(a, path).toBeCloseTo(b, 6);
  if (Array.isArray(a) && Array.isArray(b)) {
    expect(a.length, path).toBe(b.length);
    return a.forEach((one, at) => near(one, b[at], `${path}[${at}]`));
  }
  if (typeof a === "object" && a !== null && typeof b === "object" && b !== null) {
    expect(Object.keys(a).sort(), path).toEqual(Object.keys(b).sort());
    for (const key of Object.keys(a))
      near(
        (a as Record<string, unknown>)[key],
        (b as Record<string, unknown>)[key],
        `${path}.${key}`,
      );
    return;
  }
  expect(a, path).toEqual(b);
};

test("the mock's time, hours and builds come back from the wire as they went", () => {
  const now = Date.parse("2026-11-03T15:00:00Z");
  const time = jobTimeNow();
  near(timeOf(wireTime(time, now)), time);

  const drawn = usageHoursNow(now);
  const builds = usageBuildsNow(now);
  const wire = wireHours(drawn, builds);
  near(hoursOf(wire), drawn);

  const usages = new Map(
    wire.builds.map((one, at) => [
      buildIdOf(one),
      [
        ...builds,
      ].reverse()[at]!.usage!,
    ]),
  );
  const back = buildsOf(wire, usages);
  expect(back.map((one) => one.id.slice(0, 7))).toEqual(builds.map((one) => one.id));
  near(
    back.map(({ id, ...rest }) => rest),
    builds.map(({ id, ...rest }) => rest),
  );

  const one = time.jobs[0]!;
  const job = jobTimeOf(wireJobTime(time, one.id, now), undefined);
  near(job?.job.split, one.split);
  near(job?.usual, time.usual[one.workflow]);
  expect(jobTimeOf(wireJobTime(time, "01JNOTLISTED", now), undefined)).toBeUndefined();
});
