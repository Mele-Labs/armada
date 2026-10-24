// Two Macs' worth of what the Phone Gateway serves, for the fixture Gateway (`fixture/gateway.ts`) and the
// acceptance test. Times are counted back from `now`, so the ages read the same whenever it is opened.

import type { SessionRecord, SessionRow, SessionThread } from "@armada/protocol";

import type { PhoneSession } from "../gateway";

export type FixtureMac = {
  name: string;
  needs: PhoneSession[];
  live: PhoneSession[];
  threads: Record<string, SessionThread>;
  /** What `/api/sessions/shown` serves, by session and path. */
  shown: Record<string, Record<string, string>>;
  repositories: string[];
};

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

const phone = (session: PhoneSession) => session;

export function fixtureMacs(now: number, origins: { studio: string; laptop: string }): Record<string, FixtureMac> {
  const asked = phone({ id: "a1", title: "Trim the Drone pool", repository: "armada", kind: "hosted", waiting: true, working: false, started_at: ago(now, 50), waiting_since: ago(now, 12) });
  const showed = phone({ id: "a2", title: "Redraw the Needs you band", repository: "armada", kind: "hosted", waiting: false, working: true, started_at: ago(now, 34) });
  const idle = phone({ id: "a4", title: "Check the restart notes", repository: "armada", kind: "hosted", waiting: false, working: false, started_at: ago(now, 120) });
  const plan = phone({ id: "b1", title: "Rename the slot pool", repository: "store", kind: "hosted", waiting: true, working: false, started_at: ago(now, 90), waiting_since: ago(now, 40) });
  const reading = phone({ id: "b2", title: "Read the push payload", repository: "store", kind: "hosted", waiting: false, working: true, started_at: ago(now, 8) });

  const a1: SessionThread = {
    session: record(now, "a1", asked.title!, 50, {
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

  return {
    [origins.studio]: {
      name: "studio",
      needs: [asked],
      live: [asked, showed, idle],
      threads: { a1, a2 },
      shown: { a2: { "/Users/owner/armada/docs/concepts/pocket.md": "# Pocket\n\nThe phone app. It shows a Job's whole thread and any file or sketch the Job showed.\n" } },
      repositories: ["armada"],
    },
    [origins.laptop]: {
      name: "laptop",
      needs: [plan],
      live: [plan, reading],
      threads: { b1, b2 },
      shown: {},
      repositories: ["store"],
    },
  };
}
