// Two Macs' worth of what the Phone Gateway serves, for the fixture Gateway (`fixture/gateway.ts`) and the
// acceptance test. Times are counted back from `now`, so the ages read the same whenever it is opened.

import type {
  JobArtifact,
  KeptStyles,
  ModelChoices,
  PlanUsage,
  SessionRecord,
  SessionRow,
  SessionThread,
  ThreadReadyToTry,
  WaitingSource,
} from "../wire";

import { PARTS_JOB, partsArtifacts, STEP_BAR, SLOT_COLOURS, TRY_CARD, tryCard } from "./artifacts";

import type { PhoneSession, QueuedAct } from "../gateway";

export type FixtureMedia = {
  file: string;
};

export type FixtureMac = {
  name: string;
  needs: PhoneSession[];
  live: PhoneSession[];
  threads: Record<string, SessionThread>;
  /** What `/api/sessions/kept_styles` serves, by hash. */
  kept: Record<string, KeptStyles>;
  /** What `/api/sessions/shown` serves, by session and path. */
  shown: Record<string, Record<string, string>>;
  /** The pictures and videos a Job showed, by session and path: a file under `fixture/media/`, or the sentence refusing it. */
  media?: Record<string, Record<string, FixtureMedia>>;
  /**
   * A sketch the Job draws again after the phone read it, by `<session> <sketch>`: the first answer to it is refused as Fleet refuses
   * one (409, `fleet.sketch_redrawn`), and the thread then holds the new drawing, from its new `show_sketch` call.
   */
  redraws?: Record<
    string,
    {
      call: string;
      detail: Record<string, string>;
    }
  >;
  /** What `/api/sessions/artifacts` serves, by session. */
  artifacts?: Record<string, JobArtifact[]>;
  /** The Ready to try cards `/api/sessions/thread` serves, by session. */
  tries?: Record<string, ThreadReadyToTry[]>;
  repositories: string[];
  /** The Macs this one cannot reach: a dispatch naming one is kept (202), as Fleet keeps it. */
  asleep?: string[];
  /** A dispatch naming another Mac is refused as in no shared project (409). */
  unshared?: boolean;
  /** What waits for a Mac in `asleep`, oldest first. */
  queued?: QueuedAct[];
  usage: PlanUsage;
  models: ModelChoices;
};

const MODELS: ModelChoices = {
  models: [
    "opus",
    "sonnet",
    "haiku",
  ],
  default: "opus",
  session: "opus",
  read: {
    what: "read",
  },
  offered: [
    {
      name: "opus",
      efforts: [
        "low",
        "medium",
        "high",
        "xhigh",
        "max",
      ],
      on: true,
    },
    {
      name: "sonnet",
      efforts: [
        "low",
        "medium",
        "high",
      ],
      on: true,
    },
    {
      name: "haiku",
      on: true,
    },
  ],
};

const usageAt = (now: number, minutes: number, five: number, week: number): PlanUsage => ({
  read_at: ago(now, minutes),
  windows: [
    {
      kind: "five_hour",
      percent_used: five,
      band: "normal",
      resets_at: new Date(now + 3 * 3_600_000).toISOString(),
    },
    {
      kind: "seven_day",
      percent_used: week,
      band: "normal",
      resets_at: new Date(now + 2 * 86_400_000).toISOString(),
    },
  ],
});

const WINDOW = 400_000;

const ago = (now: number, minutes: number) => new Date(now - minutes * 60_000).toISOString();

const KEPT_HASH = "0123456789abcdef0123456789abcdef";
const KEPT_STYLES: KeptStyles = {
  sheets: [
    {
      path: "apps/pocket/src/styles.css",
      css: ".row { display: flex; align-items: center; gap: 8px; padding: 12px; border: 1px solid; } .lamp { inline-size: 8px; block-size: 8px; border-radius: 50%; background: currentColor; }",
      edited: false,
    },
  ],
  fonts: [],
};

const SKETCH_SCENE = {
  nodes: [
    {
      id: "phone",
      kind: "box",
      x: 0,
      y: 0,
      title: "Phone",
      body: "One app, a list per Mac",
    },
    {
      id: "studio",
      kind: "box",
      x: 260,
      y: -80,
      title: "studio",
      body: "Fleet serves the phone",
    },
    {
      id: "laptop",
      kind: "box",
      x: 260,
      y: 80,
      title: "laptop",
      body: "Fleet serves the phone",
    },
    {
      id: "card",
      kind: "mock",
      x: 600,
      y: 0,
      w: 360,
      title: "Job row",
      html: '<div class="row" data-part="row"><span class="lamp"></span>Needs you</div>',
      kept: KEPT_HASH,
    },
  ],
  edges: [
    {
      id: "a",
      from: "phone",
      to: "studio",
      label: "signed",
    },
    {
      id: "b",
      from: "phone",
      to: "laptop",
      label: "signed",
    },
  ],
};

/** The scene as the agent wrote it in its `show_sketch` call: Fleet adds `kept` to the sketch it keeps and leaves the call as it was. */
const SKETCH_CALL_SCENE = {
  ...SKETCH_SCENE,
  nodes: SKETCH_SCENE.nodes.map(({ kept: _kept, ...node }: { kept?: string }) => node),
};

function record(
  now: number,
  id: string,
  title: string,
  started: number,
  extra: Partial<SessionRecord> = {},
): SessionRecord {
  return {
    id,
    harness: "claude",
    origin: "bridge",
    manifest_id: "armada",
    cwd: "/Users/user/armada",
    title,
    state: "live",
    started_at: ago(now, started),
    last_seen_at: ago(now, 1),
    usage: {},
    attachments: [],
    hosted: {
      turn: {
        state: "idle",
      },
      mode: "auto",
      running: true,
    },
    ...extra,
  };
}

const message = (id: string, at: string, from: "you" | "agent", text: string): SessionRow => ({
  kind: "message",
  id,
  at,
  from: {
    kind: from,
  },
  text,
});
const tool = (id: string, at: string, text: string): SessionRow => ({
  kind: "tool",
  id,
  at,
  text,
});

/** A Job as the Gateway lists it; `held` defaults to false. */
const phone = (
  session: Omit<PhoneSession, "held"> & {
    held?: boolean;
  },
): PhoneSession => ({
  held: false,
  // What the Gateway sends for native's row: the last turn's instant, and Armada's Jobs under its project.
  last_turn_at: session.waiting_since ?? session.started_at,
  ...(session.repository === "armada"
    ? {
        project: "Armada",
      }
    : {}),
  ...session,
});

/** A link a hand off or a start keeps on a ledger. */
const handLink = (
  kind: string,
  target: string,
  state: "spent" | "given_back",
  at: string,
): SessionRecord["attachments"][number] => ({
  kind,
  target,
  state,
  detail: {},
  since: at,
  changed_at: at,
});

const started = (
  id: string,
  at: string,
  session: {
    id: string;
    title: string;
  },
): SessionRow => ({
  kind: "started",
  id,
  at,
  session,
  fork: false,
});

/** A Job in each state Home draws, as the list has it and as its thread has it, so a row and its Job's header can be held side by side. */
/** A Job that showed two pictures and two videos. Its own function, so `everyState`'s counts stay what the tests read. */
export function mediaJob(now: number): {
  listed: PhoneSession;
  thread: SessionThread;
  files: Record<string, FixtureMedia>;
} {
  const id = "m1";
  const title = "Draw shown pictures in the thread";
  const root = "/Users/user/armada/.playwright-mcp";
  const shown: [
    name: string,
    media: FixtureMedia,
  ][] = [
    [
      "cockpit-wide.jpg",
      {
        file: "landscape.jpg",
      },
    ],
    [
      "job-on-the-phone.jpg",
      {
        file: "portrait.jpg",
      },
    ],
    [
      "cockpit-recording.mp4",
      {
        file: "recording.mp4",
      },
    ],
    [
      "full-run.mov",
      {
        // No such file under `fixture/media`: its address holds no video, so the row draws what one that cannot play draws.
        file: "full-run.mov",
      },
    ],
  ];
  return {
    listed: phone({
      id,
      title,
      repository: "armada",
      kind: "hosted",
      waiting: false,
      working: false,
      started_at: ago(now, 30),
    }),
    thread: {
      session: record(now, id, title, 30, {
        attachments: shown.map(([name], at) => ({
          kind: "artifact",
          target: `${root}/${name}`,
          state: "standing",
          detail: {
            form: "file",
            title: name,
          },
          since: ago(now, 20 - at),
          changed_at: ago(now, 20 - at),
        })),
      }),
      rows: [
        message("m1-1", ago(now, 25), "you", "Show me what the Cockpit looks like now."),
        message(
          "m1-2",
          ago(now, 21),
          "agent",
          "The Cockpit on a wide window, and a Job on the phone.",
        ),
        tool("m1-3", ago(now, 20), `show_file ${root}/cockpit-wide.jpg`),
        tool("m1-4", ago(now, 19), `show_file ${root}/job-on-the-phone.jpg`),
        message(
          "m1-5",
          ago(now, 18),
          "agent",
          "A short recording of it running, and the whole run.",
        ),
        tool("m1-6", ago(now, 17), `show_file ${root}/cockpit-recording.mp4`),
        tool("m1-7", ago(now, 16), `show_file ${root}/full-run.mov`),
      ],
    } as SessionThread,
    files: Object.fromEntries(
      shown.map(
        ([name, media]) =>
          [
            `${root}/${name}`,
            media,
          ] as const,
      ),
    ),
  };
}

export function everyState(now: number): {
  live: PhoneSession[];
  threads: Record<string, SessionThread>;
} {
  const job = (
    id: string,
    title: string,
    started: number,
    listed: Partial<PhoneSession>,
    extra: Partial<SessionRecord>,
  ) => ({
    listed: phone({
      id,
      title,
      repository: "armada",
      kind: "hosted",
      waiting: false,
      working: false,
      started_at: ago(now, started),
      ...listed,
    }),
    thread: {
      session: record(now, id, title, started, extra),
      rows: [],
    } as SessionThread,
  });
  const hosted = (more: object) =>
    ({
      turn: {
        state: "idle",
      },
      mode: "auto",
      running: true,
      ...more,
    }) as SessionRecord["hosted"];
  const stopped = (
    reason: "turn_ended" | "process_gone",
    restart: "none" | "running" | "failed",
    more: {
      repair?: "failed";
      gave_up?: boolean;
    } = {},
  ) => ({
    listed: {
      standing: "stopped" as const,
      stopped: {
        reason,
        restart,
        ...more,
      },
    },
    extra: {
      standing: "stopped" as const,
      stopped: {
        reason,
        restart,
        at: ago(now, 5),
        ...more,
      },
    },
  });
  const blank = job(
    "x-blank",
    "",
    1,
    {
      blank: true,
    },
    {},
  );
  delete blank.listed.title;
  delete blank.thread.session.title;
  const gaveUp = stopped("process_gone", "failed", {
    repair: "failed",
    gave_up: true,
  });
  const restarting = stopped("process_gone", "running");
  const quit = stopped("turn_ended", "none");
  const jobs = [
    job(
      "x-stuck",
      "Trace the slow restart",
      41,
      {
        working: true,
        standing: "stuck",
      },
      {
        standing: "stuck",
        hosted: hosted({
          turn: {
            state: "working",
          },
        }),
      },
    ),
    job(
      "x-failing",
      "Move the pricing page",
      55,
      {
        failing_pull: 3241,
      },
      {
        attachments: [
          {
            kind: "pr",
            target: "3241",
            state: "standing",
            detail: {
              title: "Move the pricing page",
              checks: "failed",
              failing: "ci",
            },
            since: ago(now, 30),
            changed_at: ago(now, 3),
          },
        ],
      },
    ),
    job(
      "x-unstarted",
      "Port the config reader",
      63,
      {
        start_failed: "claude was not found on PATH",
      },
      {
        hosted: hosted({
          start_failed: {
            cause: "claude was not found on PATH",
          },
        }),
      },
    ),
    job("x-gave-up", "Split the ledger reads", 77, gaveUp.listed, gaveUp.extra),
    job(
      "x-queue",
      "Land the slot fixes",
      11,
      {
        held_open_by: [
          {
            kind: "pull_request",
            number: 3209,
            on: "queue",
          },
        ],
      },
      {
        held_open_by: [
          {
            kind: "pull_request",
            number: 3209,
            on: {
              kind: "queue",
            },
          },
        ],
      },
    ),
    job(
      "x-checks",
      "Fix the push retry",
      16,
      {
        held_open_by: [
          {
            kind: "pull_request",
            number: 3218,
            on: "checks",
            auto_merge: true,
          },
        ],
      },
      {
        held_open_by: [
          {
            kind: "pull_request",
            number: 3218,
            on: {
              kind: "checks",
              auto_merge: true,
            },
          },
        ],
      },
    ),
    job(
      "x-watch",
      "Deploy the store to staging",
      24,
      {
        held_open_by: [
          {
            kind: "watch",
            name: "QA deploy",
          },
          {
            kind: "pull_request",
            number: 3222,
            on: "auto_merge",
          },
        ],
      },
      {
        held_open_by: [
          {
            kind: "watch",
            name: "QA deploy",
          },
          {
            kind: "pull_request",
            number: 3222,
            on: {
              kind: "auto_merge",
            },
          },
        ],
      },
    ),
    job(
      "x-job",
      "Check the docs pass",
      28,
      {
        held_open_by: [
          {
            kind: "job",
            name: "Rename the slot pool",
          },
        ],
      },
      {
        held_open_by: [
          {
            kind: "job",
            name: "Rename the slot pool",
          },
        ],
      },
    ),
    job(
      "x-limited",
      "Rewrite the import map",
      33,
      {
        limited_until: new Date(now + 2 * 60 * 60 * 1000).toISOString(),
      },
      {
        held_until: new Date(now + 2 * 60 * 60 * 1000).toISOString(),
      },
    ),
    job("x-restarting", "Tidy the doc index", 39, restarting.listed, restarting.extra),
    job("x-quit", "Sort the settings groups", 44, quit.listed, quit.extra),
    job(
      "x-paused",
      "Sort the palette rows",
      95,
      {
        paused: true,
      },
      {
        hosted: hosted({
          paused: {
            at: ago(now, 20),
          },
        }),
      },
    ),
    job(
      "x-reading",
      "Add the billing export",
      1,
      {
        reading: true,
      },
      {
        proposal: {
          reading: true,
        },
      },
    ),
    blank,
  ];
  return {
    live: jobs.map((one) => one.listed),
    threads: Object.fromEntries(
      jobs.map((one) => [
        one.listed.id,
        one.thread,
      ]),
    ),
  };
}

export function fixtureMacs(
  now: number,
  origins: {
    studio: string;
    laptop: string;
  },
): Record<string, FixtureMac> {
  const asked = phone({
    id: "a1",
    title: "Trim the Drone pool",
    repository: "armada",
    kind: "hosted",
    waiting: true,
    working: false,
    started_at: ago(now, 50),
    waiting_since: ago(now, 12),
    context_tokens: 182_000,
    context_window: WINDOW,
  });
  const showed = phone({
    id: "a2",
    title: "Redraw the Needs you band",
    pull: 3412,
    step: "review",
    now: "Reading the band's tooltips",
    repository: "armada",
    kind: "hosted",
    waiting: false,
    working: true,
    started_at: ago(now, 34),
    context_tokens: 96_400,
    context_window: WINDOW,
  });
  const idle = phone({
    id: "a4",
    title: "Check the restart notes",
    repository: "armada",
    kind: "hosted",
    waiting: false,
    working: false,
    started_at: ago(now, 120),
  });
  const plan = phone({
    id: "b1",
    title: "Rename the slot pool",
    repository: "store",
    kind: "hosted",
    waiting: true,
    working: false,
    started_at: ago(now, 90),
    waiting_since: ago(now, 40),
  });
  const reading = phone({
    id: "b2",
    title: "Read the push payload",
    issue: {
      number: "armada-lm02",
      title: "Push payload names the Job",
    },
    repository: "store",
    kind: "hosted",
    waiting: false,
    working: true,
    started_at: ago(now, 8),
  });

  // A draft Job, held on its start card; a Job started from another; one waiting on a command; one waiting on a draft.
  const draft = phone({
    id: "d1",
    title: "Split the pool crate",
    repository: "armada",
    kind: "hosted",
    held: true,
    waiting: true,
    working: false,
    started_at: ago(now, 4),
    waiting_since: ago(now, 4),
  });
  const parent = phone({
    id: "p1",
    title: "Plan the phone screens",
    mark: {
      state: "running",
      lamp: "running",
      group: "working",
    },
    step: "build",
    now: "Running pnpm exec vitest run",
    repository: "armada",
    kind: "hosted",
    waiting: false,
    working: true,
    started_at: ago(now, 70),
    context_tokens: 241_000,
    context_window: WINDOW,
  });
  const child = phone({
    id: "c1",
    title: "Build the Draft screen",
    mark: {
      state: "subagent",
      lamp: "running",
      group: "working",
    },
    repository: "armada",
    kind: "hosted",
    waiting: false,
    working: true,
    started_at: ago(now, 18),
    from: {
      id: "p1",
      title: parent.title!,
    },
    context_tokens: 38_200,
    context_window: WINDOW,
  });
  // A second Job the planner started, and one the Draft screen's Job started in turn.
  const sibling = phone({
    id: "c2",
    title: "Write the Draft screen copy",
    repository: "armada",
    kind: "hosted",
    waiting: true,
    working: false,
    started_at: ago(now, 16),
    waiting_since: ago(now, 5),
    from: {
      id: "p1",
      title: parent.title!,
    },
    context_tokens: 22_800,
    context_window: WINDOW,
  });
  const grandchild = phone({
    id: "c3",
    title: "Measure the Draft screen on a phone",
    repository: "armada",
    kind: "hosted",
    waiting: false,
    working: true,
    started_at: ago(now, 9),
    from: {
      id: "c1",
      title: child.title!,
    },
    context_tokens: 12_400,
    context_window: WINDOW,
  });
  const restart = phone({
    id: "r1",
    title: "Move Fleet to the preview",
    repository: "armada",
    kind: "hosted",
    waiting: true,
    working: false,
    started_at: ago(now, 26),
    waiting_since: ago(now, 3),
    context_tokens: 64_900,
    context_window: WINDOW,
  });
  const describe = phone({
    id: "b3",
    title: "Open the push payload PR",
    repository: "store",
    kind: "hosted",
    waiting: true,
    working: false,
    started_at: ago(now, 30),
    waiting_since: ago(now, 6),
    context_tokens: 121_500,
    context_window: WINDOW,
  });

  // Fleet's own questions: how the work lands, every slot held, and four more on one Job.
  const landing = phone({
    id: "l1",
    title: "Fix the lost lease",
    repository: "armada",
    kind: "hosted",
    waiting: true,
    working: false,
    started_at: ago(now, 15),
    waiting_since: ago(now, 2),
  });
  const queued = phone({
    id: "s1",
    title: "Move the walk engine",
    repository: "armada",
    kind: "hosted",
    waiting: true,
    working: false,
    started_at: ago(now, 7),
    waiting_since: ago(now, 1),
  });
  const closing = phone({
    id: "o1",
    title: "Close out the tracker mod",
    repository: "armada",
    kind: "hosted",
    waiting: true,
    working: false,
    started_at: ago(now, 200),
    waiting_since: ago(now, 20),
  });

  const finished = phone({
    id: "e1",
    title: "Trim the Drone pool tests",
    repository: "armada",
    kind: "hosted",
    waiting: false,
    working: false,
    started_at: ago(now, 150),
    done_since: ago(now, 9),
    done_summary: "The pool tests now cover shrinking to four slots; nothing else changed.",
    context_tokens: 71_000,
    context_window: WINDOW,
  });

  // Two Jobs waiting on another: one on a pull request and not approved, one approved, on a part of a plan, with Fleet's question open.
  const after = phone({
    id: "w1",
    title: "Draw the slot pool on the phone",
    repository: "armada",
    kind: "hosted",
    waiting: false,
    working: false,
    started_at: ago(now, 22),
    starts_when: {
      on: {
        id: "p1",
        title: parent.title!,
      },
      when: "lands",
      pull_number: 3232,
    },
  });
  const afterPlan = phone({
    id: "w2",
    title: "Move the slot docs",
    repository: "store",
    kind: "hosted",
    waiting: false,
    working: false,
    started_at: ago(now, 35),
    starts_when: {
      on: {
        id: "b1",
        title: plan.title!,
      },
      when: "plan",
      part_title: "Rename in the crate",
    },
  });

  const w1: SessionThread = {
    session: record(now, "w1", after.title!, 22, {
      waits: {
        on: {
          id: "p1",
          title: parent.title!,
        },
        condition: {
          kind: "pr_lands",
        },
        base: "main",
        needs: "The fields the phone reads for a slot",
        brief:
          "Draw the slot pool on the phone: one row a slot, with what holds it and for how long.\n\nUse the fields the Job this waits on adds to the list. Keep the rows the Cockpit's.",
        pull: {
          number: 3232,
          branch: "armada/session-p1",
        },
        by: "you",
        held: true,
      },
    }),
    rows: [],
  };
  const w2: SessionThread = {
    session: record(now, "w2", afterPlan.title!, 35, {
      waits: {
        on: {
          id: "b1",
          title: plan.title!,
        },
        condition: {
          kind: "group_done",
          group: "g1",
        },
        base: "main",
        brief: "Move every page that says pool to the new name, once the crate has it.",
        part_title: "Rename in the crate",
        by: "agent",
        held: false,
      },
      waiting_for: [
        {
          id: "delivery",
          text: "How should this work be delivered?",
          since: ago(now, 3),
          source: "delivery",
          act: {
            kind: "answer",
            target: "delivery",
          },
          options: [
            {
              label: "Auto land",
              description: "Open a pull request, watch ci, merge it.",
            },
            {
              label: "Open PR only",
              description: "Stop at the open pull request.",
            },
            {
              label: "Keep local",
              description: "Stop at the commit on the job's branch. Nothing is pushed.",
            },
          ],
        },
      ],
    }),
    rows: [],
  };

  const d1: SessionThread = {
    session: record(now, "d1", draft.title!, 4, {
      workflow: "build",
      waiting_for: [
        {
          id: "first_message",
          text: "Start the Job",
          since: ago(now, 4),
          source: "first_message",
          act: {
            kind: "message",
            target: "d1",
          },
        },
      ],
      hosted: {
        turn: {
          state: "idle",
        },
        mode: "auto",
        running: false,
        model: "opus",
        effort: "high",
        start: {
          asks_delivery: true,
          picked: {
            choice: "open_pr",
            from: "usual",
          },
          message:
            "Split crates/pool into two crates: lease, which hands out slots, and sweep, which gives back the idle ones.\n\nKeep the public names. Move the tests with the code they cover.",
        },
      },
    }),
    rows: [],
  };

  const p1: SessionThread = {
    session: record(now, "p1", parent.title!, 70, {
      hosted: {
        turn: {
          state: "working",
        },
        mode: "auto",
        running: true,
        model: "opus",
        effort: "high",
      },
      usage: {
        context_tokens: 241_000,
        context_window: WINDOW,
      },
    }),
    rows: [
      message(
        "p1-1",
        ago(now, 70),
        "you",
        "Plan the phone screens from the sketch and start a Job for each.",
      ),
      message("p1-2", ago(now, 52), "agent", "Three screens. The Draft screen goes first."),
      started("p1-3", ago(now, 50), {
        id: "c0",
        title: child.title!,
      }),
      // c0 was handed on to c1 for context: one Job to the owner.
      {
        kind: "linked",
        id: "p1-5",
        at: ago(now, 18),
        session: {
          id: "c1",
          title: child.title!,
        },
        event: "started",
        replaces: "c0",
      },
      tool("p1-4", ago(now, 2), "Read apps/pocket/src/Home.tsx"),
    ],
  };
  p1.session.attachments = [
    handLink("started_to", "c0", "given_back", ago(now, 50)),
    handLink("started_to", "c1", "spent", ago(now, 18)),
  ];

  /** The Job c1 took the place of when it was handed on: ended, its turns read above c1's. */
  const c0: SessionThread = {
    session: record(now, "c0", child.title!, 50, {
      state: "ended",
      attachments: [
        handLink("replaced_by", "c1", "spent", ago(now, 18)),
      ],
      started: {
        by: {
          id: "p1",
          title: parent.title!,
        },
      },
    }),
    rows: [
      message(
        "c0-1",
        ago(now, 50),
        "you",
        "Build the Draft screen: the brief in a box, Send and Discard.",
      ),
      tool("c0-2", ago(now, 40), "Read apps/pocket/src/Home.tsx"),
      message(
        "c0-3",
        ago(now, 20),
        "agent",
        "The box and Send are in. Discard is next, on branch pocket/draft.",
      ),
    ],
  };

  const c1: SessionThread = {
    session: record(now, "c1", child.title!, 18, {
      hosted: {
        turn: {
          state: "working",
        },
        mode: "auto",
        running: true,
        model: "sonnet",
        effort: "medium",
      },
      usage: {
        context_tokens: 38_200,
        context_window: WINDOW,
      },
      started: {
        by: {
          id: "p1",
          title: parent.title!,
        },
      },
      attachments: [
        handLink("replaces", "c0", "spent", ago(now, 18)),
      ],
    }),
    rows: [
      {
        kind: "message",
        id: "c1-1",
        at: ago(now, 18),
        from: {
          kind: "session",
          id: "fleet",
          title: "Fleet",
        },
        text: "Carry on with the work of the job you took the place of. What it left you, with its brief if it wrote one, is in your instructions.",
        notice: "context_handoff",
      },
      tool("c1-2", ago(now, 1), "Edit apps/pocket/src/Draft.tsx"),
    ],
  };

  const r1: SessionThread = {
    session: record(now, "r1", restart.title!, 26, {
      usage: {
        context_tokens: 64_900,
        context_window: WINDOW,
      },
      waiting_for: [
        {
          id: "restart",
          text: "Restart Fleet onto the preview",
          since: ago(now, 3),
          source: "agent",
          act: {
            kind: "run",
            target: "scripts/preview --restart",
          },
        },
      ],
    }),
    rows: [
      message("r1-1", ago(now, 26), "you", "Put the slot pool fix in front of me."),
      tool("r1-2", ago(now, 8), "Bash scripts/preview"),
      message("r1-3", ago(now, 3), "agent", "The preview is built. Fleet has to restart onto it."),
    ],
  };

  const l1: SessionThread = {
    session: record(now, "l1", landing.title!, 15, {
      waiting_for: [
        {
          id: "delivery",
          text: "How should this work be delivered?",
          since: ago(now, 2),
          source: "delivery",
          act: {
            kind: "answer",
            target: "delivery",
          },
          options: [
            {
              label: "Auto land",
              description: "Open a pull request, watch ci, merge it.",
            },
            {
              label: "Open PR only",
              description: "Stop at the open pull request.",
            },
            {
              label: "Keep local",
              description: "Stop at the commit on the job's branch. Nothing is pushed.",
            },
          ],
        },
      ],
    }),
    rows: [
      message("l1-1", ago(now, 15), "you", "A slot loses its lease when Fleet restarts."),
      message(
        "l1-2",
        ago(now, 2),
        "agent",
        "Found it: the lease row is read before the store opens.",
      ),
    ],
  };

  const s1: SessionThread = {
    session: record(now, "s1", queued.title!, 7, {
      waiting_for: [
        {
          id: "slot-choice",
          text: "Every slot is in use",
          since: ago(now, 1),
          source: "slot_choice",
          act: {
            kind: "answer",
            target: "slot-choice",
          },
          options: [
            {
              label: "Wait for a slot",
            },
            {
              label: "Add a slot",
            },
          ],
        },
      ],
    }),
    rows: [
      message("s1-1", ago(now, 7), "you", "Move the walk engine into its own package."),
    ],
  };

  const o1: SessionThread = {
    session: record(now, "o1", closing.title!, 200, {
      waiting_for: [
        {
          id: "task_done:tracker",
          text: "Mark the tracker tasks done?",
          since: ago(now, 20),
          source: "task_done",
          act: {
            kind: "answer",
            target: "task_done:tracker",
          },
          options: [
            {
              label: "Mark done",
            },
            {
              label: "Keep open",
            },
          ],
          operator_read: {
            read: "Every task in the tracker mod's plan is ticked, and its pull request merged.",
            pick: "Mark done",
          },
        },
        {
          id: "rule_act:41",
          text: "Move ARM-41 to In progress",
          since: ago(now, 18),
          source: "rule_act",
          act: {
            kind: "answer",
            target: "rule_act:41",
          },
          options: [
            {
              label: "Do it",
            },
            {
              label: "Skip",
            },
          ],
        },
        {
          id: "watch_not_ours:w1",
          text: "QA deploy: smoke failed",
          since: ago(now, 12),
          source: "watch_not_ours",
          act: {
            kind: "answer",
            target: "w1",
          },
          options: [
            {
              label: "Ignore",
            },
            {
              label: "Fix it",
            },
          ],
        },
        {
          id: "words:tickets:armada",
          text: "Where does tickets open in armada?",
          since: ago(now, 5),
          source: "words_address",
          act: {
            kind: "answer",
            target: "words:tickets:armada",
          },
          options: [
            {
              label: "https://tracker.example/{match}",
              description: "Used by store",
            },
            {
              label: "Other",
            },
          ],
        },
      ],
    }),
    rows: [
      message("o1-1", ago(now, 200), "you", "Finish the tracker mod and close its tasks."),
      message("o1-2", ago(now, 21), "agent", "The mod is merged."),
    ],
  };

  const e1: SessionThread = {
    session: record(now, "e1", finished.title!, 150, {
      done_since: ago(now, 9),
      done_summary: finished.done_summary!,
      usage: {
        context_tokens: 71_000,
        context_window: WINDOW,
      },
    }),
    rows: [
      message("e1-1", ago(now, 150), "you", "Cover shrinking the pool in the tests."),
      message("e1-2", ago(now, 10), "agent", "The pool tests now cover shrinking to four slots."),
    ],
  };

  const a1: SessionThread = {
    session: record(now, "a1", asked.title!, 50, {
      usage: {
        context_tokens: 182_000,
        context_window: WINDOW,
      },
      hosted: {
        turn: {
          state: "idle",
        },
        mode: "auto",
        running: true,
        asked: {
          call: "call-a1",
          manifest_id: "armada",
          asked_at: ago(now, 12),
          tool: "AskUserQuestion",
          detail: "",
          truncated: false,
          rule: "",
          offers: [
            "allow_once",
            "refuse",
          ],
          holding_for_seconds: 3600,
          questions: [
            {
              question: "Which slots go when the pool shrinks?",
              header: "Pool",
              multi_select: false,
              options: [
                {
                  label: "The idle ones",
                  description: "Slots with no lease for an hour are given back first.",
                },
                {
                  label: "The oldest ones",
                  description: "Slots are given back in the order they were made.",
                },
              ],
            },
          ],
        },
      },
    }),
    rows: [
      message(
        "a1-1",
        ago(now, 50),
        "you",
        "The pool holds eight slots and the machine has room for four. Trim it.",
      ),
      message(
        "a1-2",
        ago(now, 46),
        "agent",
        "The pool is read in `crates/pool/src/lease.rs`. Eight slots are made at start, and none is ever given back.",
      ),
      tool("a1-3", ago(now, 40), "Read crates/pool/src/lease.rs"),
      message(
        "a1-4",
        ago(now, 13),
        "agent",
        "Two ways to choose which slots go. I need you to pick one.",
      ),
    ],
  };

  const a2: SessionThread = {
    session: record(now, "a2", showed.title!, 34, {
      hosted: {
        turn: {
          state: "working",
        },
        mode: "auto",
        running: true,
      },
      usage: {
        context_tokens: 96_400,
        context_window: WINDOW,
      },
      attachments: [
        {
          kind: "artifact",
          target: "/Users/user/armada/docs/concepts/pocket.md",
          state: "standing",
          detail: {
            form: "file",
            title: "pocket.md",
          },
          since: ago(now, 20),
          changed_at: ago(now, 20),
        },
        {
          kind: "artifact",
          target: "sketch:Phone home",
          state: "standing",
          detail: {
            form: "sketch",
            title: "Phone home",
            scene: JSON.stringify(SKETCH_SCENE),
          },
          since: ago(now, 15),
          changed_at: ago(now, 15),
        },
        {
          kind: "artifact",
          target: "http://127.0.0.1:5173/walk",
          state: "standing",
          detail: {
            form: "window",
            title: "Needs you band",
          },
          since: ago(now, 6),
          changed_at: ago(now, 6),
        },
      ],
    }),
    rows: [
      message(
        "a2-1",
        ago(now, 34),
        "you",
        "The Needs you band is too tall on the phone. Redraw it.",
      ),
      message("a2-2", ago(now, 22), "agent", "Here is what the phone says about itself now."),
      tool("a2-3", ago(now, 20), "show_file docs/concepts/pocket.md"),
      message("a2-4", ago(now, 16), "agent", "And how the phone reaches each Mac."),
      tool(
        "a2-5",
        ago(now, 15),
        `show_sketch ${JSON.stringify({
          title: "Phone home",
          scene: SKETCH_CALL_SCENE,
        })}`,
      ),
      {
        kind: "window",
        id: "a2-6",
        at: ago(now, 6),
        title: "Needs you band",
        url: "http://127.0.0.1:5173/walk",
      },
      tool("a2-7", ago(now, 2), "Edit apps/pocket/src/pocket.css"),
    ],
  };

  const c2: SessionThread = {
    session: record(now, "c2", sibling.title!, 16, {
      started: {
        by: {
          id: "p1",
          title: parent.title!,
        },
      },
      waiting_for: [
        {
          id: "copy",
          text: "Pick the Draft screen's empty line",
          since: ago(now, 5),
          source: "agent",
          act: {
            kind: "answer",
            target: "copy",
          },
          options: [
            {
              label: "No text",
            },
            {
              label: "Nothing drafted",
            },
          ],
        },
      ],
    }),
    rows: [
      message("c2-1", ago(now, 16), "agent", "Writing the Draft screen's copy."),
    ],
  };
  const c3: SessionThread = {
    session: record(now, "c3", grandchild.title!, 9, {
      hosted: {
        turn: {
          state: "working",
        },
        mode: "auto",
        running: true,
      },
      started: {
        by: {
          id: "c1",
          title: child.title!,
        },
      },
    }),
    rows: [
      message("c3-1", ago(now, 9), "agent", "Measuring the Draft screen at 390 wide."),
    ],
  };

  const b1: SessionThread = {
    session: record(now, "b1", plan.title!, 90, {
      manifest_id: "store",
      cwd: "/Users/user/store",
      waiting_for: [
        {
          id: "plan",
          text: "Approve the plan",
          since: ago(now, 40),
          source: "plan",
        },
      ],
      plan: {
        version: 1,
        summary:
          "The pool is called slots everywhere: in the crate, in what Bridge says and in the docs. Nothing behaves differently; a slot is the same thing it was.",
        recorded_at: ago(now, 40),
        approved: false,
        scope: [
          "crates/pool",
          "apps/desktop/src",
          "docs/concepts",
        ],
        groups: [
          {
            id: "g1",
            agent: "rust",
            title: "Rename in the crate",
            gives: "The crate and its tests say slot",
            paths: [
              "crates/pool",
            ],
            lands_in: {
              label: "PR 1",
            },
            gate: "checks_green",
            tasks: [
              {
                id: "t1",
                text: "Rename Pool to Slots",
                state: "open",
              },
              {
                id: "t2",
                text: "Rename the tests",
                state: "open",
              },
            ],
          },
          {
            id: "g2",
            agent: "bridge-engineer",
            title: "Rename in Bridge",
            gives: "Every screen says slot where it said pool",
            paths: [
              "apps/desktop/src",
            ],
            lands_in: {
              label: "PR 1",
            },
            gate: "walk_approved",
            tasks: [
              {
                id: "t3",
                text: "Rename the labels and tooltips",
                state: "open",
              },
              {
                id: "t4",
                text: "Walk the Slots page",
                state: "open",
              },
            ],
          },
          {
            id: "g3",
            agent: "docs-writer",
            title: "Rename in the docs",
            gives: "The docs say slot, with the old word named once",
            paths: [
              "docs/concepts",
            ],
            lands_in: {
              label: "PR 2",
            },
            own_job: true,
            gate: "merged",
            tasks: [
              {
                id: "t5",
                text: "Rename the concept page",
                state: "open",
              },
            ],
          },
        ],
      },
    }),
    rows: [
      message("b1-1", ago(now, 90), "you", "Call the pool slots everywhere."),
      {
        kind: "plan",
        id: "b1-2",
        at: ago(now, 40),
        version: 1,
      },
    ],
  };

  // A Job longer than one read: the phone opens on its newest rows and reads the rest as he scrolls up.
  const long = phone({
    id: "b4",
    title: "Trace every slot lease",
    repository: "store",
    kind: "hosted",
    waiting: false,
    working: false,
    started_at: ago(now, 300),
  });
  const b4: SessionThread = {
    session: record(now, "b4", long.title!, 300, {
      manifest_id: "store",
    }),
    rows: Array.from(
      {
        length: 450,
      },
      (_, at) =>
        at % 3 === 0
          ? message(
              `b4-${at}`,
              ago(now, 300 - at * 0.6),
              at % 6 === 0 ? "you" : "agent",
              `Turn ${at + 1} of 450.`,
            )
          : tool(`b4-${at}`, ago(now, 300 - at * 0.6), `Read crates/pool/src/lease_${at}.rs`),
    ),
  };

  const b2: SessionThread = {
    session: record(now, "b2", reading.title!, 8, {
      manifest_id: "store",
      hosted: {
        turn: {
          state: "working",
        },
        mode: "auto",
        running: true,
      },
    }),
    rows: [
      message("b2-1", ago(now, 8), "you", "Read the push payload and say what it carries."),
    ],
  };

  const DESCRIPTION =
    "Push payload carries the Job id\n\nA push now names the Job it is about, so a tap opens that Job.\n\n- `crates/pocket` adds `session_id` to the payload.\n- `public/sw.js` opens `/jobs/_/<id>` from it.";
  const b3: SessionThread = {
    session: record(now, "b3", describe.title!, 30, {
      manifest_id: "store",
      cwd: "/Users/user/store",
      usage: {
        context_tokens: 121_500,
        context_window: WINDOW,
      },
      waiting_for: [
        {
          id: "draft:b3-3",
          text: "Pull request description",
          since: ago(now, 6),
          source: "draft",
          act: {
            kind: "answer",
            target: "draft:b3-3",
          },
          options: [
            {
              label: "Approve",
            },
          ],
        },
      ],
    }),
    rows: [
      message("b3-1", ago(now, 30), "you", "Open the PR for the push payload."),
      message("b3-2", ago(now, 7), "agent", "Here is the description."),
      {
        kind: "draft",
        id: "b3-3",
        at: ago(now, 6),
        title: "Pull request description",
        text: DESCRIPTION,
      },
    ],
  };

  // A Job that waits on every kind of thing the phone answers or opens: two documents, two sketches, a workflow change, Press Start,
  // a Fleet call, a walk and a question in it.
  const review = phone({
    id: "f1",
    title: "Review what a Job waits on",
    repository: "armada",
    kind: "hosted",
    waiting: true,
    working: false,
    started_at: ago(now, 60),
    waiting_since: ago(now, 9),
    context_tokens: 88_000,
    context_window: WINDOW,
  });
  const PLAIN_PAGE = "/Users/user/armada/.armada/documents/slot-pool.html";
  const WIDE_PAGE = "/Users/user/armada/.armada/documents/phone-reach.html";
  const PLAIN_R1 = "/Users/user/.armada/kept/f1/slot-pool-r1.html";
  const WIDE_R1 = "/Users/user/.armada/kept/f1/phone-reach-r1.html";
  const WIDE_R2 = "/Users/user/.armada/kept/f1/phone-reach-r2.html";
  const PLAIN_HTML = `<!doctype html><html><head><title>Slot pool</title><link rel="stylesheet" href="armada-kit.css"></head><body>
<h1>Slot pool</h1>
<p>Four slots are enough for this machine, and the pool gives the idle ones back first.</p>
<h2>What changes</h2>
<ul><li>The pool is made with four slots, not eight.</li><li>A slot with no lease for an hour is given back.</li><li>The oldest slot is never chosen over an idle one.</li></ul>
<h2>What stays</h2>
<p>A held slot is never taken, and a Job waiting for one keeps its place in the line.</p>
</body></html>`;
  const WIDE_HTML = (
    rows: number,
  ) => `<!doctype html><html><head><title>Phone reach</title><link rel="stylesheet" href="armada-kit.css"></head><body>
<h1>Phone reach</h1>
<p>The phone reaches each Mac on its own origin, signed with a key made for that Mac.</p>
<h2>Routes by Mac</h2>
<table><caption>What each Mac answers</caption><thead><tr><th>Route</th><th>Method</th><th>Fleet route</th><th>Signed</th><th>Body</th><th>Answers</th><th>Refused with</th><th>Read by</th></tr></thead><tbody>
${Array.from(
  {
    length: rows,
  },
  (_, at) =>
    `<tr><td>/api/sessions/route-${String(at + 1)}</td><td>POST</td><td>route_${String(at + 1)}</td><td>yes</td><td>one JSON object</td><td>the Job record</td><td>409 or 422</td><td>the phone's app</td></tr>`,
).join("\n")}
</tbody></table>
<h2>How it is drawn</h2>
<armada-sketch title="Phone home"><script type="application/json">${JSON.stringify(SKETCH_CALL_SCENE)}</script></armada-sketch>
<p>Each Mac keeps its own style sets, so a frame is drawn with the ones its Mac kept.</p>
</body></html>`;
  const REDRAWN_SCENE = (boxes: string[]) => ({
    nodes: boxes.map((body, at) => ({
      id: `n${String(at)}`,
      kind: "box",
      x: at * 240,
      y: 0,
      body,
    })),
    edges: boxes.slice(1).map((_, at) => ({
      id: `e${String(at)}`,
      from: `n${String(at)}`,
      to: `n${String(at + 1)}`,
    })),
  });
  const LAYOUT_V1 = REDRAWN_SCENE([
    "Pool",
    "Eight slots",
  ]);
  const LAYOUT_V2 = REDRAWN_SCENE([
    "Pool",
    "Four slots",
    "Idle given back",
  ]);
  const asking = (id: string, text: string, source: WaitingSource, target: string) => ({
    id,
    text,
    since: ago(now, 9),
    source,
    act: {
      kind: "message" as const,
      target,
    },
  });
  const f1: SessionThread = {
    session: record(now, "f1", review.title!, 60, {
      usage: {
        context_tokens: 88_000,
        context_window: WINDOW,
      },
      attachments: [
        {
          kind: "artifact",
          target: PLAIN_PAGE,
          state: "standing",
          detail: {
            form: "document",
            title: "Slot pool",
            revisions: JSON.stringify([
              PLAIN_R1,
            ]),
            revision: "1",
            asked: "true",
            kept_at: ago(now, 30),
          },
          since: ago(now, 30),
          changed_at: ago(now, 30),
        },
        {
          kind: "artifact",
          target: WIDE_PAGE,
          state: "standing",
          detail: {
            form: "document",
            title: "Phone reach",
            revisions: JSON.stringify([
              WIDE_R1,
              WIDE_R2,
            ]),
            revision: "2",
            asked: "true",
            kept_at: ago(now, 24),
            kept_styles: JSON.stringify({
              [`Phone home#card`]: KEPT_HASH,
            }),
          },
          since: ago(now, 40),
          changed_at: ago(now, 24),
        },
        {
          kind: "artifact",
          target: "sketch:Phone home",
          state: "standing",
          detail: {
            form: "sketch",
            title: "Phone home",
            scene: JSON.stringify(SKETCH_SCENE),
            asked: "true",
            drawn: "phone-home-1",
          },
          since: ago(now, 20),
          changed_at: ago(now, 20),
        },
        {
          kind: "artifact",
          target: "sketch:Slot layout",
          state: "standing",
          detail: {
            form: "sketch",
            title: "Slot layout",
            scene: JSON.stringify(LAYOUT_V1),
            asked: "true",
            drawn: "slot-layout-1",
          },
          since: ago(now, 16),
          changed_at: ago(now, 16),
        },
        {
          kind: "artifact",
          target: "http://127.0.0.1:5173/walk",
          state: "standing",
          detail: {
            form: "window",
            title: "Pool band",
          },
          since: ago(now, 6),
          changed_at: ago(now, 6),
        },
      ],
      waiting_for: [
        asking("document:" + PLAIN_PAGE, "Slot pool", "document", "f1"),
        asking("document:" + WIDE_PAGE, "Phone reach", "document", "f1"),
        asking("sketch:Phone home", "Phone home", "sketch", "f1"),
        asking("sketch:Slot layout", "Slot layout", "sketch", "f1"),
        {
          id: "draft:f1-10",
          text: "Check before every walk",
          since: ago(now, 8),
          source: "draft",
          act: {
            kind: "answer",
            target: "draft:f1-10",
          },
          options: [
            {
              label: "Approve",
            },
            {
              label: "Dismiss",
            },
          ],
        },
        {
          id: "browser_ask:tab-1",
          text: "localhost wants to use your camera",
          since: ago(now, 7),
          source: "browser_ask",
          act: {
            kind: "answer",
            target: "browser_ask:tab-1",
          },
          options: [
            {
              label: "Allow",
            },
            {
              label: "Block",
            },
          ],
        },
        asking("start:d1", `Press Start on "${draft.title!}"`, "held_child", "d1"),
        asking("call:1", "main went red after this Job's last push", "fleet_call", "f1"),
        asking("walk:http://127.0.0.1:5173/walk", "Pool band", "walk", "f1"),
        asking("window_ask:1", "Does the band read as one line or two?", "window_ask", "f1"),
      ],
    }),
    rows: [
      message(
        "f1-1",
        ago(now, 60),
        "you",
        "Draw what each Job waits on, and ask me where it needs me.",
      ),
      started("f1-2", ago(now, 55), {
        id: "d1",
        title: draft.title!,
      }),
      message("f1-3", ago(now, 31), "agent", "The slot pool, written out."),
      message("f1-4", ago(now, 25), "agent", "How the phone reaches each Mac, revised once."),
      tool(
        "f1-5",
        ago(now, 20),
        `show_sketch ${JSON.stringify({
          title: "Phone home",
          scene: SKETCH_CALL_SCENE,
        })}`,
      ),
      tool(
        "f1-6",
        ago(now, 16),
        `show_sketch ${JSON.stringify({
          title: "Slot layout",
          scene: LAYOUT_V1,
        })}`,
      ),
      {
        kind: "window",
        id: "f1-7",
        at: ago(now, 6),
        title: "Pool band",
        url: "http://127.0.0.1:5173/walk",
      },
      message("f1-9", ago(now, 8), "agent", "One change to the build workflow, for you to save."),
      {
        kind: "draft",
        id: "f1-10",
        at: ago(now, 8),
        title: "Check before every walk",
        text: "Adds a Checks step before Walk in the build workflow, so a walk is only shown on a green build.",
        change: 7,
      },
    ],
  };

  // A Job of the phone sub project that asks the rest of To review's forms: a thing to do, a command to run, and its own
  // item asking for his approval.
  const toReview = phone({
    id: "rv",
    title: "Pocket To review",
    repository: "armada",
    kind: "hosted",
    waiting: true,
    working: false,
    started_at: ago(now, 40),
    waiting_since: ago(now, 5),
  });
  const rv: SessionThread = {
    session: record(now, "rv", toReview.title!, 40, {
      waiting_for: [
        {
          id: "approve-try",
          text: "Does the Try step look right?",
          since: ago(now, 5),
          source: "agent",
          approves: true,
          options: [
            {
              label: "Approve",
            },
            {
              label: "Ask for changes",
            },
          ],
          why: "The build waits on this.",
        },
        {
          id: "pair-phone",
          text: "Pair the phone with the laptop",
          since: ago(now, 4),
          source: "agent",
          steps: [
            "Open Pocket on the phone",
            "Menu, then Pair a Mac",
            "Scan the code the laptop shows",
          ],
        },
        {
          id: "run-check",
          text: "Run the Pocket checks",
          since: ago(now, 3),
          source: "agent",
          act: {
            kind: "run",
            target: "pnpm -C apps/pocket test",
          },
        },
      ],
    }),
    rows: [
      message("rv-1", ago(now, 40), "you", "Build the To review screen for the phone."),
      message("rv-2", ago(now, 5), "agent", "The Try step is built. Three things need you."),
    ],
  };

  return {
    [origins.studio]: {
      name: "studio",
      needs: [
        review,
        toReview,
        draft,
        sibling,
        restart,
        asked,
        landing,
        queued,
        closing,
      ],
      live: [
        review,
        toReview,
        draft,
        asked,
        restart,
        landing,
        queued,
        closing,
        parent,
        child,
        grandchild,
        sibling,
        showed,
        after,
        finished,
        idle,
      ],
      threads: {
        w1,
        rv,
        a1,
        a2,
        d1,
        f1,
        p1,
        c0,
        c1,
        c2,
        c3,
        r1,
        l1,
        s1,
        o1,
        e1,
      },
      usage: usageAt(now, 2, 42, 61),
      models: MODELS,
      kept: {
        [KEPT_HASH]: KEPT_STYLES,
      },
      shown: {
        a2: {
          "/Users/user/armada/docs/concepts/pocket.md":
            "# Pocket\n\nThe phone app. It shows a Job's whole thread and any file or sketch the Job showed.\n",
        },
        f1: {
          [PLAIN_R1]: PLAIN_HTML,
          [WIDE_R1]: WIDE_HTML(3),
          [WIDE_R2]: WIDE_HTML(6),
        },
      },
      redraws: {
        "f1 sketch:Slot layout": {
          call: `show_sketch ${JSON.stringify({
            title: "Slot layout",
            scene: LAYOUT_V2,
          })}`,
          detail: {
            scene: JSON.stringify(LAYOUT_V2),
            drawn: "slot-layout-2",
          },
        },
      },
      repositories: [
        "armada",
      ],
    },
    [origins.laptop]: {
      name: "laptop",
      needs: [
        plan,
        describe,
      ],
      live: [
        plan,
        describe,
        reading,
        long,
        afterPlan,
      ],
      threads: {
        w2,
        b1,
        b2,
        b4,
        b3,
      },
      kept: {},
      shown: {},
      repositories: [
        "store",
      ],
      usage: usageAt(now, 30, 40, 60),
      models: MODELS,
    },
  };
}

/**
 * Marks every Job the list would draw as asking him, as Fleet marks it, as the fixture server lists them: a waiting one, a
 * stuck one and one stopped for good are Needs you. One held, parked or at work is none. Done on the server's copy only, so
 * the tests' lists stay as they are.
 */
export function markWaiting(jobs: PhoneSession[]): void {
  for (const job of jobs) {
    if (job.mark !== undefined) continue;
    const parked =
      job.held ||
      job.paused === true ||
      job.starts_when !== undefined ||
      job.start_failed !== undefined ||
      job.limited_until !== undefined;
    if (parked) continue;
    const state =
      job.stopped !== undefined
        ? job.stopped.gave_up === true
          ? "stopped_asks"
          : undefined
        : job.standing === "stuck"
          ? "stuck"
          : job.waiting && !job.working
            ? "waiting"
            : undefined;
    if (state !== undefined)
      job.mark = {
        state,
        lamp: "needs",
        group: "needs",
      };
  }
}

/**
 * Jobs marked as Fleet marks them for the list's Needs you and Ready for you groups: a question and a command to run, a
 * build to try and a walk, and one at work. Added to the studio Mac by the fixture server only, so the list the tests count
 * is unchanged.
 */
export function markedJobs(now: number): PhoneSession[] {
  const needs = {
    state: "waiting",
    lamp: "needs",
    group: "needs",
  } as const;
  const ready = {
    state: "ready",
    lamp: "ready",
    group: "ready",
  } as const;
  const base = {
    repository: "armada",
    kind: "hosted",
    working: false,
  } as const;
  return [
    phone({
      ...base,
      id: "m1",
      title: "Pick the retry budget for pushes",
      mark: needs,
      owes: [
        "answer",
      ],
      waiting: true,
      started_at: ago(now, 64),
      waiting_since: ago(now, 6),
      ask: {
        ask_id: "m1-ask",
        tool: "AskUserQuestion",
        questions: [
          {
            question: "How many times should a push retry?",
            multi_select: false,
            options: [
              {
                label: "Three",
              },
              {
                label: "Five",
              },
            ],
          },
        ],
        offers: [],
      },
    }),
    phone({
      ...base,
      id: "m2",
      title: "Restart the preview on the new build",
      mark: needs,
      owes: [
        "run",
      ],
      waiting: true,
      started_at: ago(now, 41),
      waiting_since: ago(now, 19),
    }),
    phone({
      ...base,
      id: "m3",
      title: "Settings page, Account tab",
      mark: ready,
      owes: [
        "try",
      ],
      pull: 4012,
      waiting: true,
      started_at: ago(now, 88),
      waiting_since: ago(now, 3),
    }),
    phone({
      ...base,
      id: "m4",
      title: "Redraw the Job card's lamp",
      mark: ready,
      owes: [
        "walk",
      ],
      waiting: true,
      started_at: ago(now, 52),
      waiting_since: ago(now, 27),
    }),
    phone({
      ...base,
      id: "m5",
      title: "Index the librarian's gists",
      mark: {
        state: "working",
        lamp: "running",
        group: "working",
      },
      step: "build",
      now: "Running pnpm exec vitest run",
      working: true,
      waiting: false,
      started_at: ago(now, 22),
    }),
  ];
}

/**
 * A Job whose artifact holds one part of every kind and still asks, one it answered, and a build ready to try. Added to
 * the studio Mac by the fixture server, and by the tests that read it, so the list the other tests count is unchanged.
 */
export function partsJob(now: number): {
  listed: PhoneSession;
  thread: SessionThread;
  artifacts: JobArtifact[];
  tries: ThreadReadyToTry[];
} {
  const asking = (id: string, text: string, source: WaitingSource) => ({
    id,
    text,
    since: ago(now, 9),
    source,
    act: {
      kind: "message" as const,
      target: PARTS_JOB,
    },
  });
  // The Job, waiting on the artifact and the card.
  const parts = phone({
    id: PARTS_JOB,
    title: "Pick the step bar",
    repository: "armada",
    kind: "hosted",
    waiting: true,
    working: false,
    started_at: ago(now, 50),
    waiting_since: ago(now, 3),
    context_tokens: 64_000,
    context_window: WINDOW,
  });
  const f2: SessionThread = {
    session: record(now, PARTS_JOB, parts.title!, 50, {
      attachments: [
        {
          kind: "artifact",
          target: STEP_BAR,
          state: "standing",
          detail: {
            form: "sketch",
            title: "Step bar",
            asked: "true",
          },
          since: ago(now, 14),
          changed_at: ago(now, 6),
        },
        {
          kind: "artifact",
          target: SLOT_COLOURS,
          state: "standing",
          detail: {
            form: "sketch",
            title: "Slot colours",
            settled: "approved",
          },
          since: ago(now, 40),
          changed_at: ago(now, 35),
        },
      ],
      waiting_for: [
        {
          ...asking(STEP_BAR, "Step bar", "sketch"),
          since: ago(now, 6),
        },
        {
          ...asking(TRY_CARD, "Try: Steps drawn as a bar under each Job's title", "ready_to_try"),
          since: ago(now, 3),
        },
      ],
    }),
    rows: [
      message(
        `${PARTS_JOB}-1`,
        ago(now, 50),
        "you",
        "Show each Job's steps, so I can see where it is at a glance.",
      ),
      message(`${PARTS_JOB}-2`, ago(now, 41), "agent", "First, which colour says a slot is free."),
      message(
        `${PARTS_JOB}-3`,
        ago(now, 30),
        "agent",
        "Built the free colour. Now three ways to draw the steps.",
      ),
      message(
        `${PARTS_JOB}-4`,
        ago(now, 4),
        "agent",
        "The bar is on the branch. Try it in your preview.",
      ),
    ],
  };
  return {
    listed: parts,
    thread: f2,
    artifacts: partsArtifacts(now),
    tries: [
      tryCard(now),
    ],
  };
}

/** A sketch of several shapes: a start, a decision, a store, a process, a block of code, a group and a lane, in colours and with steps. */
const TOUR_SCENE = {
  nodes: [
    {
      id: "g",
      kind: "group",
      x: -40,
      y: -60,
      w: 880,
      h: 360,
      title: "Slot lifecycle",
    },
    {
      id: "lease",
      kind: "box",
      shape: "pill",
      x: 0,
      y: 0,
      title: "Lease a slot",
      body: "A Job asks for a bay",
      color: "blue",
      step: 1,
    },
    {
      id: "free",
      kind: "box",
      shape: "diamond",
      x: 300,
      y: 0,
      title: "Is one free?",
      body: "Held slots do not count",
      step: 2,
    },
    {
      id: "pool",
      kind: "box",
      shape: "cylinder",
      x: 600,
      y: -20,
      title: "Slot pool",
      body: "Eight bays on this Mac",
      color: "teal",
    },
    {
      id: "wait",
      kind: "box",
      shape: "hexagon",
      x: 300,
      y: 180,
      title: "Wait in line",
      body: "Oldest request first",
      color: "amber",
      step: 3,
    },
    {
      id: "code",
      kind: "code",
      x: 600,
      y: 160,
      lang: "toml",
      title: "settings.toml",
      body: "[slots]\ncount = 8",
    },
  ],
  edges: [
    {
      id: "e1",
      from: "lease",
      to: "free",
    },
    {
      id: "e2",
      from: "free",
      to: "pool",
      label: "yes",
      color: "green",
    },
    {
      id: "e3",
      from: "free",
      to: "wait",
      label: "no",
      color: "amber",
    },
    {
      id: "e4",
      from: "wait",
      to: "code",
    },
  ],
};

/**
 * A Job that showed every kind of artifact once: a sketch waiting on him, a document he approved, a draft he sent back, a
 * page shown for reading, and a sketch archived as stale. Added to the studio Mac by the fixture server alone, so the
 * Artifacts screen at `/jobs/<studio>/g7/artifacts` has a list to draw and each row a screen to open.
 */
export const ARTIFACTS_JOB = "g7";

export function artifactsJob(now: number): {
  listed: PhoneSession;
  thread: SessionThread;
  artifacts: JobArtifact[];
  shown: Record<string, string>;
} {
  const AUDIT = "/Users/user/armada/.armada/documents/colour-audit.html";
  const NOTES = "/Users/user/armada/.armada/documents/colour-notes.html";
  const TABLE = "/Users/user/armada/.armada/documents/slot-colour-table.html";
  const TABLE_R1 = "/Users/user/.armada/kept/g7/slot-colour-table-r1.html";
  const TABLE_R2 = "/Users/user/.armada/kept/g7/slot-colour-table-r2.html";
  const TABLE_PAGE = (rows: number) =>
    `<!doctype html><html><head><title>Slot colour table</title><link rel="stylesheet" href="armada-kit.css"></head><body><h1>Slot colour table</h1><p>Every colour a slot can wear, with the token that names it and where the phone shows it.</p><h2>Colours in use</h2><table><caption>Slot colours</caption><thead><tr><th>Slot</th><th>Token</th><th>Where it shows</th></tr></thead><tbody>${Array.from(
      {
        length: rows,
      },
      (_, at) =>
        `<tr><td>slot-${String(at + 1)}</td><td>--slot-${String(at + 1)}</td><td>The row's lamp on the Jobs list</td></tr>`,
    ).join(
      "",
    )}</tbody></table><h2>What changes</h2><p>The greys go, and green says a slot is free.</p><h3>Open</h3><p>Nobody has said what a held slot looks like.</p></body></html>`;
  const AUDIT_R1 = "/Users/user/.armada/kept/g7/colour-audit-r1.html";
  const NOTES_R1 = "/Users/user/.armada/kept/g7/colour-notes-r1.html";
  const page = (title: string, line: string) =>
    `<!doctype html><html><head><title>${title}</title><link rel="stylesheet" href="armada-kit.css"></head><body><h1>${title}</h1><p>${line}</p></body></html>`;
  const revision = (number: number, minutes: number, form: "sketch" | "draft" | "page") => ({
    number,
    at: ago(now, minutes),
    by: "agent" as const,
    form,
  });
  const listed = phone({
    id: ARTIFACTS_JOB,
    title: "Retire the old slot colours",
    repository: "armada",
    kind: "hosted",
    waiting: true,
    working: false,
    started_at: ago(now, 300),
    waiting_since: ago(now, 12),
    context_tokens: 120_000,
    context_window: WINDOW,
  });
  const thread: SessionThread = {
    session: record(now, ARTIFACTS_JOB, listed.title!, 300, {
      attachments: [
        {
          kind: "artifact",
          target: "sketch:Old slot colours",
          state: "standing",
          detail: {
            form: "sketch",
            title: "Old slot colours",
            scene: JSON.stringify(SKETCH_SCENE),
            settled: "approved",
            settled_at: ago(now, 270),
            drawn: "old-slot-colours-1",
          },
          since: ago(now, 280),
          changed_at: ago(now, 270),
        },
        {
          kind: "artifact",
          target: AUDIT,
          state: "standing",
          detail: {
            form: "document",
            title: "Colour audit",
            revisions: JSON.stringify([
              AUDIT_R1,
            ]),
            revision: "1",
            asked: "true",
            settled: "approved",
            settled_at: ago(now, 140),
            kept_at: ago(now, 150),
          },
          since: ago(now, 150),
          changed_at: ago(now, 140),
        },
        {
          kind: "artifact",
          target: NOTES,
          state: "standing",
          detail: {
            form: "document",
            title: "Colour notes",
            revisions: JSON.stringify([
              NOTES_R1,
            ]),
            revision: "1",
            kept_at: ago(now, 90),
          },
          since: ago(now, 90),
          changed_at: ago(now, 90),
        },
        {
          kind: "artifact",
          target: "sketch:Slot lifecycle",
          state: "standing",
          detail: {
            form: "sketch",
            title: "Slot lifecycle",
            scene: JSON.stringify(TOUR_SCENE),
            drawn: "slot-lifecycle-1",
          },
          since: ago(now, 80),
          changed_at: ago(now, 80),
        },
        {
          kind: "artifact",
          target: TABLE,
          state: "standing",
          detail: {
            form: "document",
            title: "Slot colour table",
            revisions: JSON.stringify([
              TABLE_R1,
              TABLE_R2,
            ]),
            revision: "2",
            asked: "true",
            kept_at: ago(now, 20),
          },
          since: ago(now, 70),
          changed_at: ago(now, 20),
        },
        {
          kind: "artifact",
          target: "sketch:Slot colours v2",
          state: "standing",
          detail: {
            form: "sketch",
            title: "Slot colours v2",
            scene: JSON.stringify(SKETCH_SCENE),
            asked: "true",
            drawn: "slot-colours-v2-2",
          },
          since: ago(now, 60),
          changed_at: ago(now, 12),
        },
      ],
      waiting_for: [
        {
          id: "sketch:Slot colours v2",
          text: "Slot colours v2",
          since: ago(now, 12),
          source: "sketch",
          act: {
            kind: "message" as const,
            target: ARTIFACTS_JOB,
          },
        },
        {
          id: `document:${TABLE}`,
          text: "Slot colour table",
          since: ago(now, 20),
          source: "document",
          act: {
            kind: "message" as const,
            target: ARTIFACTS_JOB,
          },
        },
        {
          id: `draft:${ARTIFACTS_JOB}-7`,
          text: "Slot pool description",
          since: ago(now, 8),
          source: "draft",
          act: {
            kind: "answer" as const,
            target: `draft:${ARTIFACTS_JOB}-7`,
          },
          options: [
            {
              label: "Approve",
            },
          ],
        },
      ],
    }),
    rows: [
      message(`${ARTIFACTS_JOB}-1`, ago(now, 300), "you", "Retire the old slot colours."),
      tool(
        `${ARTIFACTS_JOB}-2`,
        ago(now, 280),
        `show_sketch ${JSON.stringify({
          title: "Old slot colours",
          scene: SKETCH_CALL_SCENE,
        })}`,
      ),
      message(
        `${ARTIFACTS_JOB}-3`,
        ago(now, 150),
        "agent",
        "Every colour a slot uses, written out.",
      ),
      message(`${ARTIFACTS_JOB}-4`, ago(now, 90), "agent", "Notes on the greys, for reading."),
      tool(
        `${ARTIFACTS_JOB}-8`,
        ago(now, 80),
        `show_sketch ${JSON.stringify({
          title: "Slot lifecycle",
          scene: TOUR_SCENE,
        })}`,
      ),
      {
        kind: "draft",
        id: `${ARTIFACTS_JOB}-5`,
        at: ago(now, 40),
        title: "Check before every walk",
        text: "Adds a Checks step before Walk in the build workflow.",
        settled: {
          how: "sent_back",
          words: "Say which checks.",
          at: ago(now, 35),
        },
      },
      tool(
        `${ARTIFACTS_JOB}-6`,
        ago(now, 12),
        `show_sketch ${JSON.stringify({
          title: "Slot colours v2",
          scene: SKETCH_CALL_SCENE,
        })}`,
      ),
      {
        kind: "draft",
        id: `${ARTIFACTS_JOB}-7`,
        at: ago(now, 8),
        title: "Slot pool description",
        text: "Says how many bays a Mac holds and who may lease one.",
      },
    ],
  };
  return {
    listed,
    thread,
    shown: {
      [AUDIT_R1]: page(
        "Colour audit",
        "Eight slot colours are in use, and three of them are close.",
      ),
      [NOTES_R1]: page("Colour notes", "Grey is held, green is free, and nothing else is grey."),
      [TABLE_R1]: TABLE_PAGE(3),
      [TABLE_R2]: TABLE_PAGE(5),
    },
    artifacts: [
      {
        id: "sketch:Slot colours v2",
        title: "Slot colours v2",
        form: "sketch",
        revisions: [
          revision(1, 60, "sketch"),
          revision(2, 12, "sketch"),
        ],
        state: "asking",
        item: "sketch:Slot colours v2",
        for_line: {
          text: "Picks the five colours a slot can wear.",
          links: [],
        },
      },
      {
        id: TABLE,
        title: "Slot colour table",
        form: "page",
        revisions: [
          revision(1, 70, "page"),
          revision(2, 20, "page"),
        ],
        state: "asking",
        item: `document:${TABLE}`,
        for_line: {
          text: "Lists every slot colour and the token that names it.",
          links: [],
        },
      },
      {
        id: "sketch:Slot lifecycle",
        title: "Slot lifecycle",
        form: "sketch",
        revisions: [
          revision(1, 80, "sketch"),
        ],
        state: "reading",
        for_line: {
          text: "How a Job gets a slot, and what it does when none is free.",
          links: [],
        },
      },
      {
        id: `draft:Slot pool description`,
        title: "Slot pool description",
        form: "draft",
        revisions: [
          revision(1, 8, "draft"),
        ],
        state: "asking",
        item: `draft:${ARTIFACTS_JOB}-7`,
        for_line: {
          text: "Says how many bays a Mac holds.",
          links: [],
        },
      },
      {
        id: `draft:Check before every walk`,
        title: "Check before every walk",
        form: "draft",
        revisions: [
          revision(1, 40, "draft"),
        ],
        state: "sent_back",
        words: "Say which checks.",
        for_line: {
          text: "Adds a Checks step before Walk.",
          links: [],
        },
      },
      {
        id: AUDIT,
        title: "Colour audit",
        form: "page",
        revisions: [
          revision(1, 150, "page"),
        ],
        state: "approved",
        for_line: {
          text: "Every colour a slot uses today.",
          links: [],
        },
      },
      {
        id: NOTES,
        title: "Colour notes",
        form: "page",
        revisions: [
          revision(1, 90, "page"),
        ],
        state: "reading",
      },
      {
        id: "sketch:Old slot colours",
        title: "Old slot colours",
        form: "sketch",
        revisions: [
          revision(1, 280, "sketch"),
        ],
        state: "approved",
        archived: {
          by: "owner",
          at: ago(now, 60),
        },
      },
    ],
  };
}
