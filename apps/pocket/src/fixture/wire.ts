// Two Macs' worth of what the Phone Gateway serves, for the fixture Gateway (`fixture/gateway.ts`) and the
// acceptance test. Times are counted back from `now`, so the ages read the same whenever it is opened.

import type { ModelChoices, PlanUsage, SessionRecord, SessionRow, SessionThread } from "@armada/protocol";

import type { PhoneSession } from "../gateway";

export type FixtureMac = {
  name: string;
  needs: PhoneSession[];
  live: PhoneSession[];
  threads: Record<string, SessionThread>;
  /** What `/api/sessions/shown` serves, by session and path. */
  shown: Record<string, Record<string, string>>;
  repositories: string[];
  usage: PlanUsage;
  models: ModelChoices;
};

const MODELS: ModelChoices = {
  models: ["opus", "sonnet", "haiku"],
  default: "opus",
  session: "opus",
  offered: [
    { name: "opus", efforts: ["low", "medium", "high", "xhigh", "max"], on: true },
    { name: "sonnet", efforts: ["low", "medium", "high"], on: true },
    { name: "haiku", on: true },
  ],
};

const usageAt = (now: number, minutes: number, five: number, week: number): PlanUsage => ({
  read_at: ago(now, minutes),
  windows: [
    { kind: "five_hour", percent_used: five, resets_at: new Date(now + 3 * 3_600_000).toISOString() },
    { kind: "seven_day", percent_used: week, resets_at: new Date(now + 2 * 86_400_000).toISOString() },
  ],
});

const WINDOW = 400_000;

const ago = (now: number, minutes: number) => new Date(now - minutes * 60_000).toISOString();

const SKETCH_SCENE = {
  nodes: [
    { id: "phone", kind: "box", x: 0, y: 0, title: "Phone", body: "One app, a list per Mac" },
    { id: "studio", kind: "box", x: 260, y: -80, title: "studio", body: "Fleet serves the phone" },
    { id: "laptop", kind: "box", x: 260, y: 80, title: "laptop", body: "Fleet serves the phone" },
  ],
  edges: [
    { id: "a", from: "phone", to: "studio", label: "signed" },
    { id: "b", from: "phone", to: "laptop", label: "signed" },
  ],
};

function record(now: number, id: string, title: string, started: number, extra: Partial<SessionRecord> = {}): SessionRecord {
  return {
    id,
    harness: "claude",
    origin: "bridge",
    manifest_id: "armada",
    cwd: "/Users/owner/armada",
    title,
    state: "live",
    started_at: ago(now, started),
    last_seen_at: ago(now, 1),
    usage: {},
    attachments: [],
    hosted: { turn: { state: "idle" }, mode: "auto", running: true },
    ...extra,
  };
}

const message = (id: string, at: string, from: "you" | "agent", text: string): SessionRow => ({ kind: "message", id, at, from: { kind: from }, text });
const tool = (id: string, at: string, text: string): SessionRow => ({ kind: "tool", id, at, text });

/** A Job as the Gateway lists it; `held` defaults to false. */
const phone = (session: Omit<PhoneSession, "held"> & { held?: boolean }): PhoneSession => ({ held: false, ...session });

const started = (id: string, at: string, session: { id: string; title: string }): SessionRow => ({ kind: "started", id, at, session, fork: false });

export function fixtureMacs(now: number, origins: { studio: string; laptop: string }): Record<string, FixtureMac> {
  const asked = phone({ id: "a1", title: "Trim the Drone pool", repository: "armada", kind: "hosted", waiting: true, working: false, started_at: ago(now, 50), waiting_since: ago(now, 12), context_tokens: 182_000, context_window: WINDOW });
  const showed = phone({ id: "a2", title: "Redraw the Needs you band", repository: "armada", kind: "hosted", waiting: false, working: true, started_at: ago(now, 34), context_tokens: 96_400, context_window: WINDOW });
  const idle = phone({ id: "a4", title: "Check the restart notes", repository: "armada", kind: "hosted", waiting: false, working: false, started_at: ago(now, 120) });
  const plan = phone({ id: "b1", title: "Rename the slot pool", repository: "store", kind: "hosted", waiting: true, working: false, started_at: ago(now, 90), waiting_since: ago(now, 40) });
  const reading = phone({ id: "b2", title: "Read the push payload", repository: "store", kind: "hosted", waiting: false, working: true, started_at: ago(now, 8) });

  // A draft Job, held on its start card; a Job started from another; one waiting on a command; one waiting on a draft.
  const draft = phone({ id: "d1", title: "Split the pool crate", repository: "armada", kind: "hosted", held: true, waiting: true, working: false, started_at: ago(now, 4), waiting_since: ago(now, 4) });
  const parent = phone({ id: "p1", title: "Plan the phone screens", repository: "armada", kind: "hosted", waiting: false, working: true, started_at: ago(now, 70), context_tokens: 241_000, context_window: WINDOW });
  const child = phone({ id: "c1", title: "Build the Draft screen", repository: "armada", kind: "hosted", waiting: false, working: true, started_at: ago(now, 18), from: { id: "p1", title: parent.title! }, context_tokens: 38_200, context_window: WINDOW });
  const restart = phone({ id: "r1", title: "Move Fleet to the preview", repository: "armada", kind: "hosted", waiting: true, working: false, started_at: ago(now, 26), waiting_since: ago(now, 3), context_tokens: 64_900, context_window: WINDOW });
  const describe = phone({ id: "b3", title: "Open the push payload PR", repository: "store", kind: "hosted", waiting: true, working: false, started_at: ago(now, 30), waiting_since: ago(now, 6), context_tokens: 121_500, context_window: WINDOW });

  const d1: SessionThread = {
    session: record(now, "d1", draft.title!, 4, {
      workflow: "build",
      waiting_for: [{ id: "first_message", text: "Start the Job", since: ago(now, 4), source: "first_message", act: { kind: "message", target: "d1" } }],
      hosted: {
        turn: { state: "idle" },
        mode: "auto",
        running: false,
        model: "opus",
        effort: "high",
        start: {
          asks_delivery: true,
          picked: { choice: "open_pr", from: "usual" },
          message:
            "Split crates/pool into two crates: lease, which hands out slots, and sweep, which gives back the idle ones.\n\nKeep the public names. Move the tests with the code they cover.",
        },
      },
    }),
    rows: [],
  };

  const p1: SessionThread = {
    session: record(now, "p1", parent.title!, 70, {
      hosted: { turn: { state: "working" }, mode: "auto", running: true, model: "opus", effort: "high" },
      usage: { context_tokens: 241_000, context_window: WINDOW },
    }),
    rows: [
      message("p1-1", ago(now, 70), "you", "Plan the phone screens from the sketch and start a Job for each."),
      message("p1-2", ago(now, 20), "agent", "Three screens. The Draft screen goes first."),
      started("p1-3", ago(now, 18), { id: "c1", title: child.title! }),
      tool("p1-4", ago(now, 2), "Read apps/pocket/src/Home.tsx"),
    ],
  };

  const c1: SessionThread = {
    session: record(now, "c1", child.title!, 18, {
      hosted: { turn: { state: "working" }, mode: "auto", running: true, model: "sonnet", effort: "medium" },
      usage: { context_tokens: 38_200, context_window: WINDOW },
      started: { by: { id: "p1", title: parent.title! } },
    }),
    rows: [
      message("c1-1", ago(now, 18), "you", "Build the Draft screen: the brief in a box, Send and Discard."),
      tool("c1-2", ago(now, 1), "Edit apps/pocket/src/Draft.tsx"),
    ],
  };

  const r1: SessionThread = {
    session: record(now, "r1", restart.title!, 26, {
      usage: { context_tokens: 64_900, context_window: WINDOW },
      waiting_for: [{ id: "restart", text: "Restart Fleet onto the preview", since: ago(now, 3), source: "agent", act: { kind: "run", target: "scripts/preview --restart" } }],
    }),
    rows: [
      message("r1-1", ago(now, 26), "you", "Put the slot pool fix in front of me."),
      tool("r1-2", ago(now, 8), "Bash scripts/preview"),
      message("r1-3", ago(now, 3), "agent", "The preview is built. Fleet has to restart onto it."),
    ],
  };

  const a1: SessionThread = {
    session: record(now, "a1", asked.title!, 50, {
      usage: { context_tokens: 182_000, context_window: WINDOW },
      hosted: {
        turn: { state: "idle" },
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
          offers: ["allow_once", "refuse"],
          holding_for_seconds: 3600,
          questions: [
            {
              question: "Which slots go when the pool shrinks?",
              header: "Pool",
              multi_select: false,
              options: [
                { label: "The idle ones", description: "Slots with no lease for an hour are given back first." },
                { label: "The oldest ones", description: "Slots are given back in the order they were made." },
              ],
            },
          ],
        },
      },
    }),
    rows: [
      message("a1-1", ago(now, 50), "you", "The pool holds eight slots and the machine has room for four. Trim it."),
      message("a1-2", ago(now, 46), "agent", "The pool is read in `crates/pool/src/lease.rs`. Eight slots are made at start, and none is ever given back."),
      tool("a1-3", ago(now, 40), "Read crates/pool/src/lease.rs"),
      message("a1-4", ago(now, 13), "agent", "Two ways to choose which slots go. I need you to pick one."),
    ],
  };

  const a2: SessionThread = {
    session: record(now, "a2", showed.title!, 34, {
      hosted: { turn: { state: "working" }, mode: "auto", running: true },
      usage: { context_tokens: 96_400, context_window: WINDOW },
      attachments: [
        { kind: "artifact", target: "/Users/owner/armada/docs/concepts/pocket.md", state: "standing", detail: { form: "file", title: "pocket.md" }, since: ago(now, 20), changed_at: ago(now, 20) },
        { kind: "artifact", target: "sketch:Phone home", state: "standing", detail: { form: "sketch", title: "Phone home", scene: JSON.stringify(SKETCH_SCENE) }, since: ago(now, 15), changed_at: ago(now, 15) },
        { kind: "artifact", target: "http://127.0.0.1:5173/walk", state: "standing", detail: { form: "window", title: "Needs you band" }, since: ago(now, 6), changed_at: ago(now, 6) },
      ],
    }),
    rows: [
      message("a2-1", ago(now, 34), "you", "The Needs you band is too tall on the phone. Redraw it."),
      message("a2-2", ago(now, 22), "agent", "Here is what the phone says about itself now."),
      tool("a2-3", ago(now, 20), "show_file docs/concepts/pocket.md"),
      message("a2-4", ago(now, 16), "agent", "And how the phone reaches each Mac."),
      tool("a2-5", ago(now, 15), `show_sketch ${JSON.stringify({ title: "Phone home", scene: SKETCH_SCENE })}`),
      { kind: "window", id: "a2-6", at: ago(now, 6), title: "Needs you band", url: "http://127.0.0.1:5173/walk" },
      tool("a2-7", ago(now, 2), "Edit apps/pocket/src/pocket.css"),
    ],
  };

  const b1: SessionThread = {
    session: record(now, "b1", plan.title!, 90, {
      manifest_id: "store",
      cwd: "/Users/owner/store",
      waiting_for: [{ id: "plan", text: "Approve the plan", since: ago(now, 40), source: "plan" }],
      plan: {
        version: 1,
        summary: "The pool is called slots everywhere.",
        recorded_at: ago(now, 40),
        approved: false,
        scope: ["crates/pool"],
        groups: [
          {
            id: "g1",
            agent: "rust",
            title: "Rename in the crate",
            paths: ["crates/pool"],
            lands_in: { label: "PR 1" },
            tasks: [
              { id: "t1", text: "Rename Pool to Slots", state: "open" },
              { id: "t2", text: "Rename the tests", state: "open" },
            ],
          },
        ],
      },
    }),
    rows: [
      message("b1-1", ago(now, 90), "you", "Call the pool slots everywhere."),
      { kind: "plan", id: "b1-2", at: ago(now, 40), version: 1 },
    ],
  };

  const b2: SessionThread = {
    session: record(now, "b2", reading.title!, 8, { manifest_id: "store", hosted: { turn: { state: "working" }, mode: "auto", running: true } }),
    rows: [message("b2-1", ago(now, 8), "you", "Read the push payload and say what it carries.")],
  };

  const DESCRIPTION =
    "Push payload carries the Job id\n\nA push now names the Job it is about, so a tap opens that Job.\n\n- `crates/pocket` adds `session_id` to the payload.\n- `public/sw.js` opens `/jobs/_/<id>` from it.";
  const b3: SessionThread = {
    session: record(now, "b3", describe.title!, 30, {
      manifest_id: "store",
      cwd: "/Users/owner/store",
      usage: { context_tokens: 121_500, context_window: WINDOW },
      waiting_for: [{ id: "draft:b3-3", text: "Pull request description", since: ago(now, 6), source: "draft", act: { kind: "answer", target: "draft:b3-3" }, options: [{ label: "Approve" }] }],
    }),
    rows: [
      message("b3-1", ago(now, 30), "you", "Open the PR for the push payload."),
      message("b3-2", ago(now, 7), "agent", "Here is the description."),
      { kind: "draft", id: "b3-3", at: ago(now, 6), title: "Pull request description", text: DESCRIPTION },
    ],
  };

  return {
    [origins.studio]: {
      name: "studio",
      needs: [draft, restart, asked],
      live: [draft, asked, restart, parent, child, showed, idle],
      threads: { a1, a2, d1, p1, c1, r1 },
      usage: usageAt(now, 2, 42, 61),
      models: MODELS,
      shown: { a2: { "/Users/owner/armada/docs/concepts/pocket.md": "# Pocket\n\nThe phone app. It shows a Job's whole thread and any file or sketch the Job showed.\n" } },
      repositories: ["armada"],
    },
    [origins.laptop]: {
      name: "laptop",
      needs: [plan, describe],
      live: [plan, describe, reading],
      threads: { b1, b2, b3 },
      shown: {},
      repositories: ["store"],
      usage: usageAt(now, 30, 40, 60),
      models: MODELS,
    },
  };
}
