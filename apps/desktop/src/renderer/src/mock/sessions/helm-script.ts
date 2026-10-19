// Helm as a hosted Session, in the mock: one per repository, pinned to the dock. Mock-only; every field
// is one Fleet would owe (`packages/screens/src/draft/sessions.ts`). The agent is a script: a request
// starts a turn that stops at each step until a walk's `later` lets time pass.

import type { SessionTag as WireTag, SessionUpload, ThoughtTarget } from "@armada/protocol";
import type { Session, SessionAttachment, SessionQuestionAnswer, SessionRow, SessionTag, SubagentThread } from "@armada/screens/src/draft/sessions";

import type { SessionsStore } from "./script";

/** What a message carried to Helm or on from it, as the wire has it. */
export type HelmCarried = { attachments?: SessionUpload[]; mentions?: WireTag[] };

/** The sessions store, with Helm's Session scripted and able to be stopped. */
export type HelmStore = SessionsStore & {
  /** Words for Helm: its turn begins. */
  ask: (text: string, carry?: HelmCarried) => void;
  /** Words that went to a Session or a Job: Fleet keeps a row for them in Helm's thread. */
  sent: (text: string, target: ThoughtTarget, carry?: HelmCarried) => void;
};

const idle = { state: "idle" } as const;

/** Helm's Session for a repository, with what an earlier conversation left in it. */
export function helmRecord(manifestId: string, repository: string): Session {
  return {
    id: `helm-${manifestId}`,
    helm: true,
    manifestId,
    title: `Helm · ${repository}`,
    turn: idle,
    lastTurnAt: new Date(Date.now() - 12 * 60_000).toISOString(),
    attachments: [],
    rows: [
      { id: "e1", at: "13:41:10", kind: "compaction", text: "Job 12 stopped on the Regression check. The Sessions page work is on #2216." },
      { id: "e2", at: "13:57:21", kind: "message", from: { kind: "you" }, text: "which pull requests are open?" },
      { id: "e3", at: "13:57:30", kind: "tool", text: "Bash gh pr list --state open" },
      { id: "e4", at: "13:57:41", kind: "tool", text: "Read docs/practices/ci.md" },
      { id: "e5", at: "13:58:02", kind: "message", from: { kind: "agent" }, text: "#2219 and #2216 are open." },
    ],
  };
}

/** What the subagent does, in order: the brief, its calls, and the report it ends on. */
const SUBAGENT: readonly (readonly ["message" | "tool", string])[] = [
  ["message", "Find every caller of Clock::now outside crates/store."],
  ["tool", "Grep Clock::now crates"],
  ["tool", "Read crates/billing/src/invoice.rs"],
  ["message", "Six callers outside the store. Reading the two in billing."],
  ["tool", "Read crates/billing/src/totals.rs"],
  ["message", "Clock::now has six callers outside crates/store. Four read it once per call; the two in billing read it twice in one function."],
];

const SUBAGENT_ID = "helm-sub1";
const TASK = "Find the other clock callers";
const PULL = { number: 2231, title: "Pin the store clock in the ledger test", branch: "helm/pin-ledger-clock" } as const;

/** Wire tags as the draft's: the kind, the id and the title are all a sent row draws. */
const tagsOf = (mentions: readonly WireTag[] | undefined): SessionTag[] => (mentions ?? []).map(({ kind, id, title, url }) => ({ kind, id, title, ...(url === undefined ? {} : { url }) }));

export function withHelm(base: SessionsStore, helmId: string, repositoryOf: (manifestId: string | undefined) => string): HelmStore {
  let clock = 0;
  let rowed = 0;
  let stage = 0;
  let reads = 0;
  const at = () => {
    clock += 9;
    return `14:${String(20 + Math.floor(clock / 60)).padStart(2, "0")}:${String(clock % 60).padStart(2, "0")}`;
  };
  const row = (make: (id: string, stamp: string) => SessionRow): SessionRow => make(`h${(rowed += 1)}`, at());
  const tool = (text: string): SessionRow => row((id, stamp) => ({ id, at: stamp, kind: "tool", text }));
  const said = (text: string): SessionRow => row((id, stamp) => ({ id, at: stamp, kind: "message", from: { kind: "agent" }, text }));
  const add = (rows: SessionRow[], attachments: SessionAttachment[] = []) =>
    base.edit(helmId, (one) => ({ ...one, rows: [...one.rows, ...rows], attachments: [...one.attachments, ...attachments] }));
  const turn = (state: "working" | "idle") => base.edit(helmId, (one) => ({ ...one, turn: state === "working" ? { state } : idle, ...(state === "idle" ? { lastTurnAt: new Date().toISOString() } : {}) }));
  const filesOf = (carry: HelmCarried | undefined) =>
    (carry?.attachments ?? []).map((upload, index) => ({
      id: `f${rowed}-${index}`,
      name: upload.name,
      ...(upload.media_type.startsWith("image/") ? { src: `data:${upload.media_type};base64,${upload.data}` } : {}),
    }));

  /** Each time time passes, the turn takes its next step: one moment a walk can stop on. */
  const steps: (() => void)[] = [
    // It writes: the slot is leased and the branch cut where it happened.
    () => {
      add(
        [
          said("The ledger test reads the wall clock in two places. Pinning both."),
          row((id, stamp) => ({ id, at: stamp, kind: "lease", slot: 8, branch: PULL.branch })),
          tool("Edit crates/store/tests/ledger.rs"),
          tool("Edit crates/store/tests/ledger.rs"),
        ],
        [
          { kind: "slot", slot: 8 },
          { kind: "branch", name: PULL.branch, slot: 8 },
        ],
      );
    },
    // A subagent reads the callers while it carries on.
    () => {
      add([tool(`Task ${TASK}`)], [{ kind: "subagent", id: SUBAGENT_ID, task: TASK, state: "running" }]);
    },
    // The subagent reports, and the agent asks which clock the test should read.
    () => {
      base.edit(helmId, (one) => ({
        ...one,
        attachments: one.attachments.map((a) => (a.kind === "subagent" ? { ...a, state: "done" as const, report: SUBAGENT[SUBAGENT.length - 1]![1] } : a)),
      }));
      add([said("Six other callers read the clock. The ledger test is the only one that needs it pinned.")]);
      base.edit(helmId, (one) => ({
        ...one,
        turn: idle,
        asked: {
          command: "Which clock should the ledger test read?",
          call: "helm-q1",
          offers: ["allow_once", "refuse"],
          questions: [
            {
              question: "Which clock should the ledger test read?",
              header: "Clock",
              multi_select: false,
              options: [
                { label: "Pinned instant", description: "A fixed instant set once in the test" },
                { label: "Clock the test advances", description: "A handle the test moves by hand" },
              ],
            },
          ],
        },
      }));
    },
  ];

  const answered = (answers: readonly SessionQuestionAnswer[] | undefined, refused: boolean) => {
    base.edit(helmId, (one) => {
      const { asked: _asked, ...rest } = one;
      return { ...rest, turn: { state: "working" } };
    });
    const chosen = (answers ?? []).map((one) => one.chosen.join(" and ")).join(", ");
    window.setTimeout(() => {
      add(
        [
          said(refused ? "Skipped. Using a pinned instant." : `Going with ${chosen}.`),
          tool("Edit crates/store/tests/ledger.rs"),
          tool("Bash cargo nextest run -p store"),
          tool(`Bash gh pr create --base main --title "${PULL.title}"`),
          said(`Opened #${PULL.number}, ${PULL.title}. The ledger test reads the pinned clock; the six other callers are unchanged.`),
        ],
        [{ kind: "pull_request", ...PULL, address: `https://forge.invalid/pull/${PULL.number}`, checks: { state: "pending" }, state: "open", auto: false }],
      );
      turn("idle");
      stage = 5;
    }, 700);
  };

  const store: HelmStore = {
    ...base,
    ask(text, carry) {
      const files = filesOf(carry);
      const tags = tagsOf(carry?.mentions);
      add([
        row((id, stamp) => ({
          id,
          at: stamp,
          kind: "message",
          from: { kind: "you" },
          text,
          ...(files.length === 0 ? {} : { files }),
          ...(tags.length === 0 ? {} : { tags }),
        })),
      ]);
      turn("working");
      if (stage > 0) {
        // Any later ask is answered at once.
        window.setTimeout(() => {
          add([tool("Read crates/store/tests/ledger.rs"), said("The change is on the pull request.")]);
          turn("idle");
        }, 700);
        return;
      }
      stage = 1;
      add([tool("Read crates/store/tests/ledger.rs"), tool("Grep Clock::now crates/store"), tool("Read crates/store/src/clock.rs")]);
    },
    sent(text, target, carry) {
      const files = filesOf(carry);
      const tags = tagsOf(carry?.mentions);
      add([
        row((id, stamp) => ({
          id,
          at: stamp,
          kind: "sent",
          text,
          target: { ...target, repository: repositoryOf(target.manifest_id) },
          ...(files.length === 0 ? {} : { files }),
          ...(tags.length === 0 ? {} : { tags }),
        })),
      ]);
    },
    stop(id) {
      if (id !== helmId) return;
      turn("idle");
    },
    later() {
      const next = steps[stage - 1];
      if (stage >= 1 && next !== undefined) {
        stage += 1;
        next();
        return;
      }
      base.later();
    },
    answer(id, answer, answers) {
      if (id !== helmId) return base.answer(id, answer, answers);
      answered(answers, answer === "refuse");
    },
    subagent: async (id, subagentId): Promise<SubagentThread | undefined> => {
      if (subagentId !== SUBAGENT_ID) return base.subagent?.(id, subagentId);
      const running = stage <= 3;
      // A running subagent shows four rows and then two more each time it is read; a finished one shows all.
      const seen = running ? Math.min(SUBAGENT.length - 1, 4 + 2 * reads) : SUBAGENT.length;
      reads += 1;
      const rows = SUBAGENT.slice(0, seen).map(([kind, text], index): SessionRow => {
        const rid = `${subagentId}-${index}`;
        const stamp = `14:${String(22 + index).padStart(2, "0")}:00`;
        return kind === "tool" ? { id: rid, at: stamp, kind: "tool", text } : { id: rid, at: stamp, kind: "message", from: { kind: index === 0 ? "you" : "agent" }, text };
      });
      return { rows, finished: !running, ...(running ? {} : { report: SUBAGENT[SUBAGENT.length - 1]![1] }) };
    },
  };
  return store;
}
