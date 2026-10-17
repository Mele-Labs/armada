// **The agent half of the review store: everything an agent does, as runs Fleet keeps.** A quick action, Ask and a review tool
// start a run (`start_review_run`) and meet its progress and result as `review_run.changed`; what is kept for a pull request is
// read when it is opened, so a result survives a restart of Bridge or of Fleet. The store (`store.tsx`) reaches it only through
// `AgentActs`. An agent runs only when he presses, and asks first: `spend` is the one gate. Natural Search (`store-search.ts`)
// and the Walk (`store-walk.ts`) are the other two halves, and both meet the run through the same `begin` and `apply`.

import type { PullReviewRow, ReviewRun, ReviewTools, WalkPlace } from "@armada/protocol";

import type { Asked, Finding, PrWork, ReviewPr, ReviewSnapshot, ToolDef } from "./model";
import { accountOf, agentRunOf, answerOf, explanationOf, findingsOf, runKey, toolDefOf, wordsOf } from "./runs";
import type { ReviewSource } from "./source";
import type { ToolIn } from "./store";

/** What the agent half reads and moves of the store. */
export type StoreHold = {
  get: () => ReviewSnapshot;
  set: (next: Partial<ReviewSnapshot>) => void;
  work: (key: string) => PrWork;
  change: (key: string, by: Partial<PrWork>) => void;
  mend: (key: string, by: Partial<ReviewPr>) => void;
  prOf: (key: string) => ReviewPr | undefined;
  nextId: (prefix: string) => string;
  /** Takes a pull request a search turned up, which may be on none of the four lists, so it opens like any other. Answers its key. */
  adopt: (row: PullReviewRow) => string;
};

export type AgentActs = {
  spend: (what: string, run: () => void, cancelled?: () => void) => void;
  confirmSpend: (again: boolean) => void;
  cancelSpend: () => void;
  explain: (key: string, def: ToolDef) => void;
  explainAgain: (key: string, def: ToolDef) => void;
  show: (key: string, kind: string) => void;
  ask: (key: string, words: string) => void;
  askAgain: (key: string, id: string) => void;
  removeAsk: (key: string, id: string) => void;
  runTool: (key: string, tool: ToolDef) => void;
  rerun: (key: string, tool: ToolDef) => void;
  dismiss: (key: string, finding: string) => void;
  sent: (key: string, findings: readonly string[]) => void;
  /** Saves a tool or explanation at Fleet. Answers Fleet's words where it would not, and nothing where it took it. */
  addTool: (one: ToolIn) => Promise<string | undefined>;
  /** Reads what is kept for a pull request: run on its commit, going, or failed. Called when it is opened and when the lists are read again. */
  readRuns: (key: string) => void;
  /** Reads a repository's tools and explanations, once. */
  readTools: (manifestId: string) => void;
  /** A run moved, whole (`review_run.changed`). A search has no pull request and is the store's to route (`store.tsx`). */
  apply: (run: ReviewRun) => void;
  /** Starts a run, or asks for the one kept. `place` is where the Walk's change runs. A refusal is the run failed, in Fleet's words. */
  begin: (key: string, kind: ReviewRun["kind"], name: string, again: boolean, place?: WalkPlace) => void;
  /** A file's patch was read or dropped: the explanation's hunks are cut from it again. */
  patched: (key: string) => void;
};

const HOW_IT_WORKS = runKey("explanation", "how-it-works");

/** A later run replaces an earlier; a run that has finished is not taken back to going by its own start arriving late. */
export const staler = (held: ReviewRun, run: ReviewRun): boolean => {
  const was = Date.parse(held.started_at);
  const now = Date.parse(run.started_at);
  if (was !== now) return was > now;
  return (held.state === "done" || held.state === "failed") && (run.state === "queued" || run.state === "running");
};

const said = wordsOf;

export function createAgentActs(source: ReviewSource, hold: StoreHold): AgentActs {
  const { get, set, work, change, mend, prOf, nextId } = hold;
  let waiting: (() => void) | undefined;
  let declined: (() => void) | undefined;
  const toolsHeld = new Set<string>();
  const toolsReading = new Set<string>();
  /** Questions he removed this session, which Fleet still keeps: they stay off the page until he asks them again. */
  const removed = new Set<string>();
  const forgotten = (key: string, words: string) => `${key}\n${words.toLowerCase()}`;

  const askedBy = (key: string, words: string): Asked | undefined => work(key).asks.find((one) => one.words.toLowerCase() === words.toLowerCase());
  const setAsk = (key: string, words: string, by: Partial<Asked>) => change(key, { asks: work(key).asks.map((one) => (one.words.toLowerCase() === words.toLowerCase() ? { ...one, ...by } : one)) });

  /** How it works is the run's answer cut from the patches read so far: empty until the run is done. */
  const cut = (key: string) => {
    const run = work(key).wire[HOW_IT_WORKS];
    mend(key, { explanation: explanationOf(run?.state === "done" ? run.result : undefined, work(key).patches) });
  };

  const apply = (run: ReviewRun) => {
    const pr = get().prs.find((one) => one.manifestId === run.manifest_id && one.number === run.number);
    if (pr === undefined) return;
    const key = pr.key;
    const rk = runKey(run.kind, run.name);
    const held = work(key).wire[rk];
    if (held !== undefined && staler(held, run)) return;
    const now = agentRunOf(run);
    const by: Partial<PrWork> = { wire: { ...work(key).wire, [rk]: run } };
    if (run.kind === "explanation") {
      by.explains = { ...work(key).explains, [run.name]: now };
      const account = run.state === "done" ? accountOf(run.result) : undefined;
      if (account !== undefined) by.accounts = { ...work(key).accounts, [run.name]: account };
    } else if (run.kind === "tool") {
      by.runs = { ...work(key).runs, [run.name]: now };
      if (run.state === "done") {
        const again = held?.started_at === run.started_at;
        const before = work(key).findings;
        const found = findingsOf(run).map((one): Finding => {
          const same = again ? before.find((it) => it.id === one.id) : undefined;
          return same === undefined ? one : { ...one, dismissed: same.dismissed, sent: same.sent };
        });
        by.findings = [...before.filter((one) => one.tool !== run.name), ...found];
      }
    } else {
      const body = run.state === "done" ? answerOf(run.result) : undefined;
      const one = askedBy(key, run.name);
      if (one !== undefined) {
        by.asks = work(key).asks.map((it) => (it === one ? { ...it, run: now, ...(body === undefined ? {} : { answer: body }) } : it));
      } else if (!removed.has(forgotten(key, run.name))) {
        by.asks = [...work(key).asks, { id: nextId("ask"), words: run.name, run: now, ...(body === undefined ? {} : { answer: body }) }];
      }
    }
    change(key, by);
    if (rk === HOW_IT_WORKS) cut(key);
  };

  /** Starts a run, or asks for the one kept: going at once on screen, then as Fleet says. A refusal is the run failed, in Fleet's words. */
  const begin = (key: string, kind: ReviewRun["kind"], name: string, again: boolean, place?: WalkPlace) => {
    const pr = prOf(key);
    if (pr === undefined) return;
    const going = { state: "running", since: Date.now() } as const;
    if (kind === "explanation") change(key, { explains: { ...work(key).explains, [name]: going } });
    else if (kind === "tool") change(key, { runs: { ...work(key).runs, [name]: going } });
    else setAsk(key, name, { run: going });
    void source.startRun(pr.manifestId, { number: pr.number, kind, name, ...(again ? { again: true } : {}), ...(place === undefined ? {} : { place }) }).then((answer) => {
      if (answer.ok) return apply(answer.value);
      const failed = { state: "failed", said: answer.said, at: Date.now(), head: pr.head } as const;
      if (kind === "explanation") change(key, { explains: { ...work(key).explains, [name]: failed } });
      else if (kind === "tool") change(key, { runs: { ...work(key).runs, [name]: failed } });
      else setAsk(key, name, { run: failed });
    });
  };

  const holdTools = (manifestId: string, value: ReviewTools) => {
    toolsHeld.add(manifestId);
    set({
      tools: { ...get().tools, [manifestId]: value.tools.map(toolDefOf) },
      leftOut: { ...get().leftOut, [manifestId]: (value.left_out ?? []).map((one) => ({ file: one.file, source: one.source, said: one.said })) },
    });
  };

  return {
    spend: (what, run, cancelled) => {
      if (!get().askFirst) return run();
      waiting = run;
      declined = cancelled;
      set({ spending: what });
    },
    confirmSpend: (again) => {
      const run = waiting;
      waiting = undefined;
      declined = undefined;
      set({ spending: undefined, ...(again ? { askFirst: false } : {}) });
      run?.();
    },
    cancelSpend: () => {
      const no = declined;
      waiting = undefined;
      declined = undefined;
      set({ spending: undefined });
      no?.();
    },
    explain: (key, def) => {
      change(key, { kind: def.id });
      const kept = work(key).explains[def.id];
      if (kept === undefined || kept.state === "idle") begin(key, "explanation", def.id, false);
    },
    explainAgain: (key, def) => {
      if (work(key).explains[def.id]?.state !== "running") begin(key, "explanation", def.id, true);
    },
    show: (key, kind) => change(key, { kind }),
    ask: (key, words) => {
      const asked = said(words);
      if (prOf(key) === undefined || asked === "") return;
      removed.delete(forgotten(key, asked));
      const had = askedBy(key, asked);
      const id = had?.id ?? nextId("ask");
      if (had === undefined) change(key, { asks: [...work(key).asks, { id, words: asked, run: { state: "idle" } }] });
      change(key, { kind: id });
      if (had?.run.state !== "running") begin(key, "ask", had?.words ?? asked, had?.run.state === "failed");
    },
    askAgain: (key, id) => {
      const one = work(key).asks.find((it) => it.id === id);
      if (one !== undefined && one.run.state !== "running") begin(key, "ask", one.words, true);
    },
    removeAsk: (key, id) => {
      const one = work(key).asks.find((it) => it.id === id);
      if (one !== undefined) removed.add(forgotten(key, one.words));
      change(key, { asks: work(key).asks.filter((it) => it.id !== id), kind: work(key).kind === id ? undefined : work(key).kind });
    },
    runTool: (key, tool) => {
      change(key, { view: tool.id });
      // The Walk is started by its place being found (`store-walk.ts`): a run needs the place.
      if (!tool.agent || tool.mark === "walk") return;
      const run = work(key).runs[tool.id];
      if (run === undefined || run.state === "idle") begin(key, "tool", tool.id, false);
    },
    rerun: (key, tool) => {
      if (tool.mark !== "walk" && work(key).runs[tool.id]?.state !== "running") begin(key, "tool", tool.id, true);
    },
    dismiss: (key, finding) => change(key, { findings: work(key).findings.map((one) => (one.id === finding ? { ...one, dismissed: true } : one)) }),
    sent: (key, findings) => change(key, { findings: work(key).findings.map((one) => (findings.includes(one.id) ? { ...one, sent: true } : one)) }),
    addTool: async ({ kind, name, checks, scope, manifestId }) => {
      const to = manifestId === "" ? get().prs[0]?.manifestId : manifestId;
      if (to === undefined) return "No repository to save to";
      const answer = await source.saveTool(to, { scope, kind, name: name.trim(), prompt: checks.trim() });
      if (!answer.ok) return answer.said;
      // A Kit tool is in every repository, so the others are read again where they are next opened.
      if (scope === "kit") toolsHeld.clear();
      holdTools(to, answer.value);
      return undefined;
    },
    readRuns: (key) => {
      const pr = prOf(key);
      if (pr === undefined) return;
      void source.runs(pr.manifestId, pr.number).then((answer) => {
        if (!answer.ok || prOf(key) === undefined) return;
        for (const run of [...answer.value.runs].reverse()) apply(run);
        // What he had open comes back open: the explanation that was read shows again.
        if (work(key).kind === undefined && work(key).explains["how-it-works"]?.state === "done") change(key, { kind: "how-it-works" });
      });
    },
    readTools: (manifestId) => {
      if (toolsHeld.has(manifestId) || toolsReading.has(manifestId)) return;
      toolsReading.add(manifestId);
      void source.tools(manifestId).then((answer) => {
        toolsReading.delete(manifestId);
        if (answer.ok) holdTools(manifestId, answer.value);
      });
    },
    apply,
    begin,
    patched: (key) => {
      if (work(key).wire[HOW_IT_WORKS]?.state === "done") cut(key);
    },
  };
}
